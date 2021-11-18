<h1 align="center">CRHS</h1>

<p align="center">
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/AUTHORS"><img src="https://img.shields.io/badge/authors-SimulaUIB-orange.svg"></a>
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>

***If you came here looking for PathFinder***: PathFinder is in the process of being made readable for outsiders, and will
be present here as soon as possible. For inquires on the process, please email johnpetter@simula.no, and I will answer
as soon as possible.

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
move more towards [Akers'](https://doi.org/10.1109/TC.1978.1675141) [2] "definition" of a BDD. It would be interesting to further explore the
eligibility of using CRHS equations in traditional fields of BDDs.

---
**WARNING:** This tool was developed in an academic context and no part of this code should be used in any production 
system. In particular the implementations of cryptosystems in this tool are not safe for any real world usage.

## License

CryptaPath is licensed under the MIT License.

* MIT license ([LICENSE](../LICENSE) or http://opensource.org/licenses/MIT)


## Overview

**CRHS** consists of four libraries: `CRUSH`, `CryptaPath`, `PathFinder` and `TBD`.  

This section will be completed as soon as the port of PathFinder is complete.


## References

1) Indrøy, John Petter, Nicolas Costes, and Håvard Raddum. "Boolean Polynomials, BDDs and CRHS Equations-Connecting the
 Dots with CryptaPath." International Conference on Selected Areas in Cryptography. Springer, Cham, 2020.
2) Akers, Sheldon B. "Binary decision diagrams." IEEE Transactions on computers 27.06 (1978): 509-516.
