# Rust Baremetal on Teensy 4.0

This project provides Rust abstractions for the NXP i.MX RT1062
processor used in the Teensy 4.0. It is intended to be usable as a
library, although currently only works for executables defined in its
own `examples` directory.

It is very early in development, and as such as more limitations than
feature.

## Limitations

* Code runs only off of flash (as does `.rodata`)
* No abstractions - all registers must be accessed directly via pointers
* No `.data` section, since we cannot copy the initialization data in yet
* RAM is used for `.bss`, but is not yet zero-initialized
* Cannot be used as a dependency crate

## Dependencies

In order to build and use this project, you'll need the following:

* A recent Nightly build of Rust, installed using `rustup`
* An Arduino environment with Teensyduino installed on top of it
* GNU Make (BSD Make variants may work, but are untested)

## Running Examples

To run the examples, set an `ARDUINO` environment variable that points
to your teensyduino installation, then run `make <examplename>`. This
will build the requested example and open the Teensy loader to flash
it. You can then simply hit the reset button on your Teensy, just as
you would in the Arduino environment.

```
$ export ARDUINO=$HOME/Downloads/arduino-1.8.9 # Or wherever your Arduino lives
$ make bootup
```

## Next Steps

In no particular order, the following hardware bits need abstractions
built for them. These will likely be similar to [what I did for Teensy
3.2](https://github.com/branan/teensy), where it makes sense.

* GPIO
* UART
* CCM
* SPI
* USB (especially USB serial for debugging)
* DMA

There are also some remaining memory management bits that need doing:

* ITCM and DTCM setup
* Copying code/data into correct memories
* BSS zero-initialization
* A heap

Lastly, a Cargo `build.rs` script needs to be written so that we force
the appropriate linker script when this crate is consumed. This will
allow its use as a library.