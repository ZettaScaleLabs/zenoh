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

## Compiling the Rust Toolchain for QNX

QNX support for Rust is not yet available via the `rustup` command. Instead, it is necessary to compile the Rust toolchain from source. The build process for the Rust toolchain is described below.

1. Ensure the QNX Software Development Platform (SDP) is installed and configured on the host machine.

2. Setup the QNX environment:
    
```bash
source ~/qnx710/qnxsdp-env.sh
```
    
3. Checkout Rust:
        
```bash
git clone --recurse-submodules https://github.com/rust-lang/rust.git
cd rust
git checkout 1.75.0
```
        
4. Configure the environment to build for QNX:
        
```bash
export build_env='
  CC_aarch64-unknown-nto-qnx710=qcc
  CFLAGS_aarch64-unknown-nto-qnx710=-Vgcc_ntoaarch64le_cxx
  CXX_aarch64-unknown-nto-qnx710=qcc
  AR_aarch64_unknown_nto_qnx710=ntoaarch64-ar
  CC_x86_64-pc-nto-qnx710=qcc
  CFLAGS_x86_64-pc-nto-qnx710=-Vgcc_ntox86_64_cxx
  CXX_x86_64-pc-nto-qnx710=qcc
  AR_x86_64_pc_nto_qnx710=ntox86_64-ar'
 ```
        
5. Configure the build with the `x.py` utility, choosing `(d) dist`:
        
```bash
./x.py setup
```
        
6. Edit the generated `config.toml` file to specify a custom install location for Rust by adding the following lines:
        
```bash
install.prefix = "<install location>"
install.sysconfdir = "<install location>/etc"
```
        
7. Build Rust for QNX x86 and aarch64:
        
```bash
env $build_env \
./x.py build \
--target aarch64-unknown-nto-qnx710,x86_64-pc-nto-qnx710
```
        
       
6. Install Rust:
        
```bash
env $build_env \
  ./x.py install \
  --target aarch64-unknown-nto-qnx710,x86_64-pc-nto-qnx710
```
        
7. Add the custom build of Rust with support for QNX to the `PATH` and `LD_LIBRARY_PATH`:
    
```bash
export PATH=<install location>/bin:$PATH
export LD_LIBRARY_PATH=<install location>/lib:$LD_LIBRARY_PATH
```
