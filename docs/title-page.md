# Namespaces in operation

This text is an adaptation of Michael Kerrisk's [series on LWN][original],
written by [Carlos Valiente][carletes] in order to learn some Rust and Linux
namespaces internals.

The code presented here is a Rust rewrite of the original author's sample
programs.

All sample progams can be built using `cargo`:

```text
$ cargo build
```

## License

The original series in LWN was made available in 2013 under the
[Creative Commons CC BY-SA 4.0][cc-by-sa/4.0] license. Since this text is a
derivative work of it, this text itself is covered by the
[Creative Commons CC BY-SA 4.0][cc-by-sa/4.0] license as well.

The original C programs from the LWN series were release by Michael Kerrisk
under the [GNU General Public License v2][gplv2]. The Rust versions of those
programs contained here are therefore released under the same
[GNU General Public License v2][gplv2] license as well.


[carletes]: https://github.com/carletes
[cc-by-sa/4.0]: https://creativecommons.org/licenses/by-sa/4.0/
[gplv2]: https://www.gnu.org/licenses/old-licenses/gpl-2.0.en.html
[original]: https://lwn.net/Articles/531114/
