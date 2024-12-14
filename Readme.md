![Logo](images/logo_small.png)

Rusterix is a fast software renderer for 2D and 3D triangles and lines. Its goals are to provide an easy and portable alternative to hardware rasterization for retro and low-poly games.

---

## How it works

Rusterix uses a multi-threaded, tile-based renderer and organizes 2D and 3D meshes into batches. It precomputes the bounding boxes and triangle edges for each batch during projection.

The main goal is to achieve a single rendering pass to maximize parallelization. The batching system makes this possible while also enabling grouping and optimizations for individual objects and content.

Because of these optimizations, Rusterix is not a general-purpose abstraction of a hardware rendering pipeline (for that, consider using the excellent [euc](https://github.com/zesterer/euc)). Instead, it features a custom pipeline specifically optimized for software rendering and operates within a fixed color space.

## Goals and Status

Once finished, you will be able to use Rusterix in several different ways:

* As a library to rasterize 2D and 3D meshes, WIP. See the `Cube` example.
* As a retro game engine with an editor for Doom style games. Programmable in Rust and an inbuild scripting system (TBD).

My goals for both of these use cases:

* Fast software based rendering.
* Procedural materials and particles for in game effects and content.

## Motivation

I use `rusterix` as the rendering engine for my [Eldiron](https://github.com/markusmoenig/Eldiron) project. But it makes sense to split it out into a re-usable library and engine.

![Logo](images/screenshot.png)

## Examples

To execute an example just do something like ```cargo run --release --example cube```.

* **cube** displays a spinning, textured cube.
* **obj** demonstrates how to load and display an obj file.


## Disclaimer

Rusterix is an independent project and is not affiliated with or endorsed by the Rust programming language team or the Rust Foundation. All trademarks are the property of their respective owners.

## License

`rusterix` is distributed under either of:

- Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at the discretion of the user.
