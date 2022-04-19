<h1 align="center">PathFinder</h1>

<p align="center">
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/AUTHORS"><img src="https://img.shields.io/badge/authors-SimulaUIB-orange.svg"></a>
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>

## About
__PathFinder__ (__PF__) is where the "post-processing" logic of a resolved differential/linear based SOC
resides. This library contains the functionality to estimate the best input-output pair to use
as basis for a hull, and then to calculate the exact values of the hull based on the paths contained in
the SOC. Note that due to the pruning process, not all possible paths may be present for the post-processing.
See the [PathFinder paper](https://doi.org/10.1007/978-3-030-81652-0_9) for more.
---
**WARNING:** This library was developed in an academic context and no part of
this code should be use in any production system.

## Overview
TBI

## Licence
__PathFinder__ is licensed under the MIT License.

* MIT license ([LICENSE](../LICENSE) or http://opensource.org/licenses/MIT)

## How to use
The easiest way to use PF is through the `SOCCS` library/binary.

## Plan for the future

- Streamline logging and reporting of results.
- Implement error handling?
- Apply Clippy and Rustfmt
- (Research topic:) Add a second pass for the hull generation: Today's logic will include all possible
input and output pairings possible to a primitive, run the SOC resolving logic (including the 
pruning) and use the result to try and find the best input-output pair to calculate the
hull for. A second pass would reset the process to pre SOC resolving, but this time resolve
only for the input-output pair found. This would in theory allow for more targeted pruning,
hopefully preserving more paths in the hull.
- (Research topic:) Improve the hull calculation: Today's logic expands every path in the hull
in order to calculate the exact values of paths in the hull. This yields a problem when the
hull size is exponentially large, as not every path can be checked then. We therefore have
an upper limit today on the number of paths expanded/included. A technique which would traverse
each edge of the solved SOC only once, instead of visiting each path once, was discussed during
the latter stages of development, but had to be dropped due to time constraints. (Main dev finishing
Ph.D.). This technique would allow for all paths in a hull to be included, independently of size.

## Known issues
TBI

## Naming
TBI