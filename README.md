<h1 align="center">CRHS</h1>

<p align="center">
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/AUTHORS"><img src="https://img.shields.io/badge/authors-SimulaUIB-orange.svg"></a>
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>

---
**CRHS** consists of the two research projects **CryptaPath** and **PathFinder**. At the core of these projects are the
Compressed Right-Hand Side (CRHS) equations; a datastructure which may be viewed as a derivation of Binary Decision 
Diagrams (BDDs). The connection between BDDs and CRHS equations are explored in [CryptaPath](https://doi.org/10.1007/978-3-030-81652-0_9) [1].

Both CryptaPath and PathFinder use CRHS equations to perform cryptanalysis on SPN based symmetric ciphers. CryptaPath
use them to launch an algebraic cryptanalytical attack, and the theory of CRHS equations were originally developed for
this purpose. CryptaPath expands on this work by "merging" CRHS equations with linear and differential cryptanalysis, to
search for good linear and differential hulls.

Despite that CRHS equations currently are solely used for cryptanalysis, we believe that some concepts introduced
by CRHS equations may be relevant for fields where BDDs are traditionally used. For example, it may be of interest that 
CRHS equations offer a solution to having linear dependencies in the decision variables of BDDs, meaning that we may 
move more towards [Akers'](https://doi.org/10.1109/TC.1978.1675141) [2] usage/meaning of a BDD. It would be interesting
to further explore the eligibility of using CRHS equations in traditional fields of BDDs.

---
**WARNING:** This tool was developed in an academic context and no part of this code should be used in any production 
system. In particular the implementations of cryptosystems in this tool are not safe for any real world usage.

## License

CryptaPath is licensed under the MIT License.

* MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)


## Workspace Overview

**CRHS** consists of four libraries: `CRUSH`, `CryptaPath`, `PathFinder` and `SOCCs`.  

The CRHS workspace consists of four libraries, each one having their own readme.

The four libraries are:
- [`CRUSH`](crush): Contains the core functionality of `CRHS equations` and `SOCs`.
- [`CryptaPath`](CryptaPath): The original library showcasing algebraic cryptanalysis using`SOCs`. See [CryptaPath](https://doi.org/10.1007/978-3-030-81652-0_9) [1]
- [`PathFinder`](pathfinder): The core logic behind the search for good linear and differential hulls using `CRHS equations` 
and `SOCs`.
- [`SOCCS`](soccs): Adapts the ciphers and trait found in [CryptaGraph](https://eprint.iacr.org/2018/764.pdf) [3]
to be used with `PathFinder` for linear and differential hull search using `CRHS equations` and `SOCs`.

## Current state
Unfortunately, the current state of all the code written as part of the `SOCCs` project is not up to standards.
The new libraries (PathFinder and SOCCS) were developed as prototypes, meaning that speed of implementation was favoured
over documentation and down-payment of technical dept. This is the sad reality of writing code as part of a Ph.D. theis.

You will therefore find that the documentation is not at the standard I would like it to be, and that the code is more
messy than I'd like to admit. As time passes, I hope to be able to work gradually on these issues. Unfortunately, I'm 
unable to make any commitments as to how often improvements will happen.

However, if you have questions, please email me at johnpetter@simula.no, and I will try to get back to you as soon as
possible.

## Build guide and usage

We target the stable channel of Rust.

To build you have first to install Rust (you can follow the guide from the [`official website`](https://www.rust-lang.org/tools/install).
If you already have Rust installed make sure that your version is at least 1.38 as we make extensive usage of std::HashMap and it was greatly improved on that patch.

You can then run:
```bash
git clone https://github.com/Simula-UiB/CRHS.git
cd CRHS
cargo build
```
This will build all the libraries in the workspace. To build a specific library only add the `-p` flag followed 
by the library name.  
For example, in order to build the `CRUSH` library, write

```bash
cargo build -p crush
```

To run any of the binaries (found in `PathFinder` and `SOCCS`), replace `build` with `run`, and then the name of the 
binary. Note that the binaries expect additional flags with the command, see the respective libraries' README for more
info.

---
You can run the unit tests using:

```bash
cargo test
``` 


## References

1) John Petter Indrøy, Nicolas Costes, and Håvard Raddum. "Boolean Polynomials, BDDs and CRHS Equations-Connecting the
 Dots with CryptaPath." International Conference on Selected Areas in Cryptography. Springer, Cham, 2020.
2) Akers, Sheldon B. "Binary decision diagrams." IEEE Transactions on computers 27.06 (1978): 509-516.
3) Mathias Hall-Andersen and Philip S. Vejre. "Generating graphs packed with paths", FSE 2019.
(Cryptology ePrint Archive 2018)