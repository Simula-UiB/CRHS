<h1 align="center">CryptaPath</h1>

<p align="center">
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/AUTHORS"><img src="https://img.shields.io/badge/authors-SimulaUIB-orange.svg"></a>
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>

__CryptaPath__ is a tool built on top of the Crush library to automate the generation of systems of Binary Decision Diagrams (BDDs) from implementations of cryptographic algorithms (ciphers using S-boxes and sponge hash functions are supported) and their solving process. The objective of this tool is to provide an easy way to evaluate the resistance of a cryptosystem to algebraic attacks.

**WARNING:** This tool was developed in an academic context and no part of this code should be used in any production system. In particular the implementations of cryptosystems in this tool are not safe for any real world usage.

**Acknowledgement** This tool was greatly inspired by the work done by Philip Vejre on [`CryptaGraph`](https://gitlab.com/psve/cryptagraph/tree/master) (you can think of the name as an hommage). Some part of the design (S-Box for example) were greatly influenced by his design.

## License

CryptaPath is licensed under the MIT License.

* MIT license ([LICENSE](../LICENSE) or http://opensource.org/licenses/MIT)


## Overview

CryptaPath provides 2 main subcommands, `cipher` and `sponge` and 2 helper commands `make-cipher-param` and `from-file`.

The `cipher` command lets you build a system of BDDs for all supported ciphers for any number of rounds and try to solve it for a randomly generated pair of plaintext/ciphertext. You can also provide your own pair of plaintext/ciphertext to build your system from. A partial value of the key you are trying to find can also be provided with its unknown and guessed (known) bits. The `make-cipher-param` command can generate those values (key, plaintext/ciphertext) for you for any cipher.

The `sponge` command lets you build a system of BDDs for the supported sponge hash for any number of rounds and any valid value of rate/capacity, hash length and max message length. You can provide your own hash value for which you want to find a preimage and any known or guessed bits of the message.

The systems generated by the tool can be output in a specific format with the `-o` option and later solved again with the `from-file` command.

## Build guide

**To be updated** to reflect that this is no longer a standalone library and binary, but rather part of a rust workspace.

We target the stable channel of Rust.

To build you have first to install Rust (you can follow the guide from the [`official website`](https://www.rust-lang.org/tools/install).
If you already have Rust installed make sure that your version is at least 1.38 as we make extensive usage of std::HashMap and it was greatly improved on that patch.

You can then run 
```bash
git clone https://github.com/Simula-UiB/CryptaPath.git
cd CryptaPath/cryptapath
cargo build --release
```

You can run the unit tests using:

```bash
cargo test
``` 

All the supported algorithms come with test vectors (their origins are provided in comments in the source code) and will be tested with this command.
The test for the LowMC cipher is ignored in debug because running this particular cipher is quite long. If you wish to run its test as well you can simply run:


```bash
cargo test --release
```

Finally to make the documentation for this library you can use

```bash
cargo doc
```

The documentation will be available in `cryptapath/target/release/doc/crush/all.html`, which you can open in your browser.


## Examples

Here are some examples of command lines that you can run with this tool (run while in cryptapath directory).

```bash
cargo run --release -- cipher -c skinny64128 -r 4
```

This will generate a random pair of plaintext/ciphertext and try to solve the system for the cipher Skinny reduced to 4 rounds, with a block size of 64 bits and key of 128 bits.

```bash
cargo run --release -- cipher -c skinny64128 -r 10 -k 0000000000000000000000000000000000000000000000000XXXX00001110X0X0100010101100010XXX0000000000000000000000000000000XX111010101010
```

This will generate a system for the cipher SKinny with 64 block size and 128 bits key reduced to 10 rounds where you know some bits of the key (in that case you know 117 bits out of 128, the X in the binary string shows the unknown bits).

```bash
cargo run --release -- sponge --capacity 160 --hash-length 80 --message-length 240 --rate 240 --rounds 1 -s keccak
```

This will generate a system for the sponge construction Keccak reduced to 1 round with a 240-bit rate, 160-bit capacity, 80-bit hash output and 240-bit max message length.


```bash
cargo run --release -- sponge --capacity 160 --hash-length 80 --message-length 240 --rate 240 --rounds 2 -s keccak --partial-preimage XXXXXXX00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000XXXX11
```

This will generate a system for the sponge construction Keccak reduced to 2 rounds with a 240-bit rate, 160-bit capacity, 80-bit hash output and 240-bit max message length where you know bits of the preimage (you know 229 bits out of 240).

A complete view of the possibilities of the tool can be found using the [`--help`] parameter on each command available.


## Adding new algorithms

All supported cryptosystems are located in [`targets`](src/targets). Currently CryptaPath supports 2 reduced version of AES (SR* 2x2x8 and SR* 4x4x4), LowMC, SKINNY, PRESENT, PRINCE, DES and Keccak. You can add new cryptosystems by implementing the `Cipher` or the `SpongeHash` trait from [`targets`](src/targets/mod.rs). For an easy example on how to do that you can look at the [`PRESENT`](src/targets/present80.rs) implementation.

## Experimenting with solving

The solvers used are implemented in [`strategy.rs`](src/strategy.rs). You can add new solvers or tweak the existing one and run them using the `-s` argument for ciphers.  This option is currently not present for `SpongeHash` since only solvers that use dropping are effective in this case.


If something is not covered in this README, please check the documentation.