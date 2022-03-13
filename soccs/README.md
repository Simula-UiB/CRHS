<h1 align="center">SOCCs</h1>

<p align="center">
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/AUTHORS"><img src="https://img.shields.io/badge/authors-SimulaUIB-orange.svg"></a>
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>

## About
__SOCCs__ is a way to quickly get started with differential and linear analysis of
SPN-based ciphers using Compressed Right-Hand Side equations (`CRHS equations`). 
The library already has 24 ciphers (including variations of same ciphers) supported, and
also provides a mean to analyse your own cipher by the means of the `Cipher` trait. To get 
started with your own cipher, all you need is to implement the required trait.

**WARNING:** This library was developed in an academic context and no part of 
this code should be use in any production system.

## Overview
TBI

## Licence
__SOCCs__ is licensed under the MIT License.

* MIT license ([LICENSE](../LICENSE) or http://opensource.org/licenses/MIT)

## How to use

### - Implement trait for own primitive
TBI

### - CLI
TBI

## Plan for the future

- Clean and simplify the logic.
- Find a way to incorporate CryptaPath into SOCCS, or vica versa.
  - The former may require some adaptions to the trait shared with CryptaGraph,
  or we need to find an easy way to have multiple traits.
- Apply Clippy and Rustfmt 

## Known issues
TBI

## Naming
A collection of `CRHS equations` where all the `CRHS equations` are related is
known as a _System of CRHS equations_, a _SOC_. All the SOCs generated
with this binary are based on cryptographic primitives, and the name thus became
__SOCCs__: __Systems of Crypto-based Compressed Right-Hand Side equations__.
