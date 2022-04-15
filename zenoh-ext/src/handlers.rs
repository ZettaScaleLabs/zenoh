use futures::FutureExt;
use zenoh::{
    prelude::Sample,
    subscriber::{
        CallbackSubscriber, CallbackSubscriberBuilder, Reliability, SubMode, SubscriberBuilder,
    },
    time::Period,
};
use zenoh_core::Result as ZResult;
use zenoh_sync::ZFuture;
use Box as Dyn;
type SenderClosure<'a, T> = Dyn<dyn Fn(T) + Send + Sync + 'a>;

/// Handlers
pub trait Handler<'a, T> {
    ///
    type Receiver: Unpin;
    fn split(self) -> (SenderClosure<'a, T>, Self::Receiver);
}

impl<'a, T, F: Fn(T) + Send + Sync + 'a> Handler<'a, T> for F {
    type Receiver = ();
    fn split(self) -> (SenderClosure<'a, T>, ()) {
        (Dyn::new(self), ())
    }
}

impl<T: Send + 'static> Handler<'static, T> for (flume::Sender<T>, flume::Receiver<T>) {
    type Receiver = flume::Receiver<T>;
    fn split(self) -> (SenderClosure<'static, T>, flume::Receiver<T>) {
        let (tx, rx) = self;
        (
            Dyn::new(move |value| {
                if let Err(e) = tx.send(value) {
                    log::warn!("Couldn't send value because receiver was closed: {}", e)
                }
            }),
            rx,
        )
    }
}

impl<T: Send + Sync + 'static> Handler<'static, T>
    for (std::sync::mpsc::SyncSender<T>, std::sync::mpsc::Receiver<T>)
{
    type Receiver = std::sync::mpsc::Receiver<T>;
    fn split(self) -> (SenderClosure<'static, T>, std::sync::mpsc::Receiver<T>) {
        let (tx, rx) = self;
        (
            Dyn::new(move |value| {
                if let Err(e) = tx.send(value) {
                    log::warn!("Couldn't send value because receiver was closed: {}", e)
                }
            }),
            rx,
        )
    }
}

pub trait SupportsHandlers<'a, CallbackInput, HandlerBuilder: Handler<'a, CallbackInput>> {
    type Receiver;
    fn handler(self, handler: HandlerBuilder) -> Self::Receiver;
}
impl<'a, 'b, HandlerBuilder: Handler<'static, Sample>>
    SupportsHandlers<'static, Sample, HandlerBuilder> for SubscriberBuilder<'a, 'b>
{
    type Receiver = SubscriptionBuilder<'a, 'b, HandlerBuilder::Receiver>;
    fn handler(self, handler: HandlerBuilder) -> Self::Receiver {
        let (tx_closure, rx) = handler.split();
        let builder = self.callback(tx_closure);
        SubscriptionBuilder {
            builder,
            receiver: Some(rx),
        }
    }
}
pub struct SubscriptionBuilder<'a, 'b, Receiver> {
    receiver: Option<Receiver>,
    builder: CallbackSubscriberBuilder<'a, 'b>,
}
impl<'a, 'b, Receiver> SubscriptionBuilder<'a, 'b, Receiver> {
    pub fn reliability(mut self, reliability: Reliability) -> Self {
        self.builder = self.builder.reliability(reliability);
        self
    }

    /// Change the subscription reliability to `Reliable`.
    #[inline]
    pub fn reliable(self) -> Self {
        self.reliability(Reliability::Reliable)
    }

    /// Change the subscription reliability to `BestEffort`.
    #[inline]
    pub fn best_effort(self) -> Self {
        self.reliability(Reliability::BestEffort)
    }

    /// Change the subscription mode.
    #[inline]
    pub fn mode(mut self, mode: SubMode) -> Self {
        self.builder = self.builder.mode(mode);
        self
    }

    /// Change the subscription mode to Push.
    #[inline]
    pub fn push_mode(self) -> Self {
        self.period(None).mode(SubMode::Push)
    }

    /// Change the subscription mode to Pull.
    #[inline]
    pub fn pull_mode(self) -> Self {
        self.mode(SubMode::Pull)
    }

    /// Change the subscription period.
    #[inline]
    pub fn period(mut self, period: Option<Period>) -> Self {
        self.builder = self.builder.period(period);
        self
    }

    /// Make the subscription local only.
    #[inline]
    pub fn local(mut self) -> Self {
        self.builder = self.builder.local();
        self
    }
}
impl<'a, 'b, Receiver: Unpin> std::future::Future for SubscriptionBuilder<'a, 'b, Receiver> {
    type Output = ZResult<Subscription<'a, Receiver>>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.builder.poll_unpin(cx).map(|r| {
            r.map(|s| Subscription {
                receiver: self
                    .receiver
                    .take()
                    .expect("overpolling of a SubscriptionBuilder"),
                subscriber: s,
            })
        })
    }
}
impl<'a, 'b, Receiver: Unpin + Send> ZFuture for SubscriptionBuilder<'a, 'b, Receiver> {
    fn wait(mut self) -> Self::Output {
        Ok(Subscription {
            receiver: self.receiver.take().unwrap(),
            subscriber: self.builder.wait()?,
        })
    }
}

#[derive(Debug)]
/// A subscriber associated with a Receiver.
pub struct Subscription<'a, Receiver> {
    pub receiver: Receiver,
    pub subscriber: CallbackSubscriber<'a>,
}
impl<'a, Receiver> std::ops::Deref for Subscription<'a, Receiver> {
    type Target = Receiver;
    fn deref(&self) -> &Self::Target {
        &self.receiver
    }
}
impl<'a, Receiver> std::ops::DerefMut for Subscription<'a, Receiver> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.receiver
    }
}
impl<'a, Receiver: Send + Unpin> Subscription<'a, Receiver> {
    pub fn undeclare(self) -> impl ZFuture<Output = ZResult<Receiver>> {
        let Subscription {
            receiver,
            subscriber,
        } = self;
        ZFutureMap {
            fut: subscriber.undeclare(),
            map: Some(move |r: ZResult<()>| r.map(move |_| receiver)),
        }
    }
}

/// A map operation for ZFutures
struct ZFutureMap<Fut: std::future::Future, Map: FnOnce(Fut::Output) -> R, R> {
    fut: Fut,
    map: Option<Map>,
}
impl<Fut: std::future::Future + Unpin, Map: FnOnce(Fut::Output) -> R + Unpin, R> std::future::Future
    for ZFutureMap<Fut, Map, R>
{
    type Output = R;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.fut.poll_unpin(cx).map(|r| self.map.take().unwrap()(r))
    }
}
impl<Fut: ZFuture + Unpin, Map: FnOnce(Fut::Output) -> R + Unpin + Send, R> ZFuture
    for ZFutureMap<Fut, Map, R>
{
    fn wait(self) -> <Self as futures::Future>::Output {
        self.map.unwrap()(self.fut.wait())
    }
}
