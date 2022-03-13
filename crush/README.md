<h1 align="center">Crush </h1>

<p align="center">
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/AUTHORS"><img src="https://img.shields.io/badge/authors-SimulaUIB-orange.svg"></a>
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>



__Crush__ provides the core functionality for working with __Compressed Right-Hand Side(`CRHS`) equations__ 
and __Systems of Compressed Right-Hand Side equations__ (`SOCs`). It is developed as part of ongoing research on `CRHS`
equations and`SOCs`.

## About
***TBI. What is it? Why is this useful? What is it used for?***  
A method to solve a system of non-linear equations over GF(2) using `CRHS` equations. (Which are based on BDDs).

*For a more thorough yet gradual treatment of `CRHS` equations and `SOCs`, see the [paper][1][^pdf].*

[1]: https://link.springer.com/chapter/10.1007/978-3-030-81652-0_9
[^pdf]: PDF available at simulamet.no: https://www.simulamet.no/sites/default/files/publications/files/cryptapath.pdf

### CRHS equations
A Compressed Right-Hand Side (`CRHS`) equation is based on a Reduced Ordered Binary Decision Diagram (`ROBDD`). It deviates
from a standard `ROBDD` on three points:

1) The decision variable for a level is "extracted", meaning that we associate the decision variable with
   its level instead of with each individual node on the level.
2) We allow linear combinations to be associated with a level, instead of a single variable. 
3) We have only one terminal node, the true node, instead of both the true and the false node.

The first is cosmetic in its nature: Since the `BDD` is ordered, we know that each variable will be encountered in the
same order when traversing any path, and thus that each variable will only exist on one level. Associating a variable
with the level instead of each node in the level will simplify notation, and it allows us to shrink the memory footprint
of a node in the code.


The second change brings the `CRHS`/`BDD` more in line with Akers' (1)[^Akers] definition of a `BDD`, rather than the more common
to use definition formulated by Bryant (2)[^Bryant]. By allowing linear combinations we also allow for a variable to appear
on multiple levels. This does not come without risk: If a path may require a variable to take the value of 1 on one level
and the value of 0 at another level, effectively yielding an invalid solution. Such paths are said to be *inconsistent*,
and are considered undesirable to keep in the `CRHS` equation and `SOC`. Akers tackled this issue by saying that special
care need to be taken when constructing the `BDD`, to avoid this issue. Bryant solved it by disallowing a variable to be
present on more than one level along any path. `CRHS` equations, on the other hand, do things differently: A novel
algorithm, known as *linear absorption*, identifies and removes inconsistent paths from the `CRHS` equation. This 
algorithm is also backwards compatible with `BDD`s, potentially allowing for new use-cases for `BDD`s (this is an avenue
we didn't have time to explore).  
The benefit of allowing linear combinations to be associated with a level, is that we may compress the structure more
than an equivalent `BDD`. I.e. a `CRHS` equation and a `BDD` representing the same Boolean equation, the `CRHS` equation will 
almost always consist of fewer nodes and/or levels. Furthermore, it allows a `CRHS` equation to represent complex 
contexts. For instance, we can build `CRHS` equations from cryptographic primitives with certain properties.  
The drawback of linear absorption is memory consumption. Depending on the underlying problem the `CRHS` equation
represents, as well as the ordering of the levels, performing a linear absorption may significantly increase the number
of nodes in the `CRHS` equation. This is particularly evident when representing cryptographic primitives, where the hunt
for the secret key becomes a race between eliminating paths faster than the paths inherent "longing" to decompress 
themselves[^1]. 

[^1]: This would be a good example, but as the concept is still work in progress I'm unable to express it cleanly. My apologies.

The third change means that we no longer represents a Boolean function, but rather a Boolean equation equal to 1.
`CRHS` equations originate from work in algebraic cryptanalysis of symmetric SPN ciphers, where paths ending in the 
false node cannot yield the secret key. Removing the false node also often result in a `CRHS` equation with fewer nodes.
However, this change is mainly cosmetic: The paper introducing linear absorption (3)[^Schilling] still had the false node
as part of the datastructure, showing that removing the false node is not strictly necessary.

[^Akers]: *Binary Decision Diagrams*, by Sheldon B. Akers. In *IEEE Transactions on computers* (1978, p. 509--516).  
[^Bryant]: *Graph-Based Algorithms for Boolean Function Manipulation*, by Randal E. Bryant. In *IEEE Transactions on* 
  *computers* (1986 p. 677--691).  
[^Schilling]: *Solving compressed right hand side equation systems with linear absorption* by T. Schilling and H. Raddum.
  In *International Conference on Sequences and Their Applications* (2012, p. 291--301)

### SOCs

A set of `CRHS` equations.  
Useful in situations where capturing/building one inital `CRHS` equation is difficult. For instance to model a SPN cipher.  
Have various algorithms which allows to iteratively join the set of `CRHS` equations into one `CRHS` equation, dealing
with new linear dependencies along the way if desired.
See papers for more. (For instance Schilling [^Schilling], CryptaPath paper and other papers by Schilling, Raddum, Indrøy)

### Size and growth of CRHS equations and SOCs
***To be written!*** *These are some notes/thoughts to get started:*

Do know that:
- For a `ROBDD` (and thus, for CRHS), the optimal order is NP-complete to find.
- "Some functions that can be represented by Boolean expressions or logic circuits of reasonable size but for all input
orderings the representation as a function graph is too large to be practical". Quote Brynt. I.e. not all "things" which
technically may be represented as a BDD is practical to be so. (Research question: Does allowing linear combinations for
`CRHS` equations improve on this?).
- See results from Lee, whereby is seems that for Boolean functions of more than a certain number of variables, a `BDD`
will always evaluate in fewer operations?


For `CRHS` equations and `SOC`s, we also know that
 - the join operations will decrease the size (number of nodes) in the resulting `CRHS` equation by 1.
 - linear absorption may worst case double the size of the `CRHS` equation.
 - the size of a `CRHS` equation is worst case 2^m, where m is the number of levels (AKA the depth) in the `CRHS` 
 equation. However, I've never seen that happening.
 - Solving a `SOC` based on a real cipher always results in running out of memory. My hypothesis is that is happens
because a symmetric cipher is bijective (under a given key), and thus that the graph seeks to "untangle"/"decompress"
itself. (I call this "becoming spaghetti"). This means that the further in the process we go, the fewer paths passes
through each node on a level, as each path seek to become a single path connecting the source to the sink. (This will
ultimately lead to as many nodes on a level as there are paths going through it). Solving a SPN based `SOC` then becomes
a race between eliminating paths and the "decompression" rate of the graph.


## License

Crush is licensed under the MIT License.

* MIT license ([LICENSE](../LICENSE) or http://opensource.org/licenses/MIT)


**WARNING:** This library was developed in an academic context and no part of this code should be use in any production system.

<!---
## Overview

This library implements a way of solving system of equations over GF(2) using Compressed Right-Hand Side equations 
(`CRHS equations`).
For this we provide 3 rust modules that can be used together : 

- [`algebra`](src/algebra): Rust module that provides operations on matrices over GF(2).
- [`soc`](src/soc): Rust module that provides a memory representation of a system of CRHS equations and the APIs to
mutate it safely with the available operations.
- [`solver`](src/solver): Rust module that provides a way of defining Solvers: Structures holding the strategy which
will use the [`soc`](soc) APIs to absorb all linear dependencies inside a System of CRHS equations (`SOC`), making every
remaining path a valid solution.
---> 

## Build guide

We target the stable channel of Rust.

To build you have first to install rust (you can follow the guide from the [`official website`](https://www.rust-lang.org/tools/install).

You can then run 
```bash
git clone https://github.com/Simula-UiB/CryptaPath.git
cd CryptaPath/Crush
cargo build --release
```

You can run the unit test for the modules [`algebra`](src/algebra) and [`soc`](src/soc) using :

```bash
cargo test
``` 

Finally to make the documentation for this library you can use

```bash
cargo doc --no-deps
```

The documentation will be available in [`target/release/doc/crush/all.html`], which you can open in your browser.
If you want the documentation for [`Node`](src/soc/node.rs) and [`Level`](src/soc/level.rs) you may add the flag `--document-private-items`.

## .bdd file format

One of the way to load a system of CRHS equations is to use a .bdd file and the function `parse_system_spec_from_file` 
from the [`utils`](src/soc/utils.rs) module. The .bdd format is a legacy format from the initial research into what is now
know as CRHS equations.

The specification for the file is as follows :

```text
nr of unique vars
nr of CRHS's in the system
crhs_id number_of_levels_in_this_crhs
LHS (a linear combination of variables id, ex: 13+3+35) : RHS (nodes and links, format: (node_id;id_to_0-edge,id_to_1-edge) )|
...
last_level (no associated left-hand side, one node with both edges pointing to nothing)
---
(next CRHS)
---
(next CRHS)
---
...
(last CRHS)
---
```

Things to note:
- ":" is the divider between the left-had side (LHS) and the right-hand side (RHS).
- "|" is the end of level marker.
- "---" is the end of bdd marker.
- "id_to_0-edge"/"id_to_1-edge" is the node_id which the 0/1-edge points to, where a node_id of 0 means that this edge
points to nothing.

## Example of the solving process

You can find an example of a complete solving process (including fixing variables and printing the solutions) in the
tool [`CryptaPath`][CryptaPath].

**Warning:** This implementation is mono-threaded and can be very heavy on RAM consumption (on big systems it can 
easily grow to 200 GB of RAM and more). Be mindful of this if you are running this on cloud engines or constrained servers.

If something was not covered in this README please check the documentation.

[crhs]: https://link.springer.com/chapter/10.1007%2F978-3-642-30615-0_27
[CryptaPath]: https://github.com/Simula-UiB/CryptaPath