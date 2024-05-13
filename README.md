# milliquad-hack

This is a somewhat hideous combination of Macroquad and Speedy2D.

When I decided to see if I could take my Speedy2D-based project and get it working with Miniquad, this repo is what happened.  All the code here came from either Speedy2D or Macroquad, but the authors of those packages should not be blamed for this.

Why did I do this?  Because I was having a great experience with Speedy2D, but I wanted to see if I could port to Miniquad to gain its exceptional cross-platform support.

The following files came from Macroquad:

- `quad_gl.rs`
- `shapes.rs`
- `text.rs`
- `texture.rs`

The rest of the files came from Speedy2D.

The Speedy2D type `Graphics2D` is the primary 2D graphics API.  It is defined in `lib.rs` but has been modified to be more Macroquad-like in certain ways.

The Speedy2D type `WindowHandler` is defined in `window.rs`.  It is similar to `EventHandler` from Miniquad, except a bit different, and with an `on_draw()` call that provides a `Graphics2D`.

The main code that bridges the two worlds is in `window_internal_quad.rs`, and this is most of the stuff I wrote, using the glutin implementation from Speedy2D as a starting point.

