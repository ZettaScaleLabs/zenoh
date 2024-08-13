<img src="https://raw.githubusercontent.com/eclipse-zenoh/zenoh/master/zenoh-dragon.png" height="150">

# Eclipse Zenoh - QNX Support

**Support for QNX 7.1 is currently a work in progress with limited functionality supported.**

To compile Zenoh for QNX 7.1 a QNX development environment and a version of the Rust toolchain supporting the QNX targets are required. Internal development testing is carried out on the `x86_64-pc-nto-qnx710` target.

## Supported Features

The following Zenoh packages and features are currently supported:

* Package: zenoh (`--no-default-features` argument must be specified)
  * auth_pubkey
  * auth_usrpwd
  * transport_multilink
  * transport_compression
  * transport_tcp
  * transport_udp
  * transport_ws
* Package: zenoh-ext

To build all of the above packages and features for the `x86_64-pc-nto-qnx710` target the command would be:

```
cargo build --target x86_64-pc-nto-qnx710 -p zenoh --no-default-features --features auth_pubkey,auth_usrpwd,transport_multilink,transport_compression,transport_tcp,transport_udp,transport_ws -p zenoh-ext
```

## Supported Examples

The following Zenoh examples are supported:

* z_scout
* z_info
* z_put
* z_put_float
* z_delete
* z_formats
* z_pub
* z_sub
* z_pull
* z_queryable
* z_storage
* z_get
* z_forward
* z_pub_thr/z_sub_thr
* z_ping/z_pong

To build the examples for the `x86_64-pc-nto-qnx710` target the command would be:

```
cargo build --target x86_64-pc-nto-qnx710 -p zenoh-examples --examples
```
