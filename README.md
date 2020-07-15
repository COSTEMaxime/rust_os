# rust_os
Small project to learn more about Rust and OS. Based on: https://os.phil-opp.com/

# Install

```
$ sudo apt install cargo
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ rustup override add nightly
$ cargo install cargo-xbuild
$ cargo install bootimage
$ rustup component add llvm-tools-preview
```

This will do the following :
- Install cargo
- Install rustup
- Switch to rust nightly
- Install bootimage (tool ued to build the bootloader)

## pc-keyoard driver
Go to the parent directory of the project and download this repository [pc-keyboard](https://github.com/COSTEMaxime/pc-keyboard.git) (Ansi fr keyboard mapping).

## GUI Install (WSL only)
If your'e using Windows subsystel for Linux you'll need to install an X server to run graphical application. I'm using [VcXsrv](https://sourceforge.net/projects/vcxsrv/).

Install it on windows then add the following to your .bashrc :
```
$ echo "export DISPLAY=localhost:0.0" >> ~/.bashrc
```

# Build
```
$ cargo xbuild
```

# Test
```
$ cargo xtest
```

# Run
```
$ cargo xrun
```