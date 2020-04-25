# Preface

This text has been compiled by [Carlos Valiente][carletes] in order to
learn some Rust and Linux namespaces internals.

This text is an mostly an adaptation of a [series on LWN][original] on
Linux kernel namespaces. The original articles on LWN were written by
Michael Kerrisk and Jake Edge

The chapter on network namespaces is mostly an adaptation of [Ifeanyi
Ubah's post][ifeanyi-post].

The code presented here is a Rust rewrite of the original authors' sample
programs.  All sample progams can be built using `cargo`:

```text
$ cargo build
```

## License

The original series in LWN was made available between 2013 and 2016 under
the [Creative Commons CC BY-SA 4.0][cc-by-sa/4.0] license. Since this text
is a derivative work of it, this text itself is covered by the [Creative
Commons CC BY-SA 4.0][cc-by-sa/4.0] license as well.

The original C programs from the LWN series were release by Michael Kerrisk
under the [GNU General Public License v2][gplv2]. The Rust versions of
those programs contained here are therefore released under the same [GNU
General Public License v2][gplv2] license as well.

[Ifeanyi's code on network namespaces][ifeanyi-code] is released under the
[MIT license][mit]. The Rust version of that code is therefore released
under the same [MIT license][mit].


[carletes]: https://github.com/carletes
[cc-by-sa/4.0]: https://creativecommons.org/licenses/by-sa/4.0/
[gplv2]: https://www.gnu.org/licenses/old-licenses/gpl-2.0.en.html
[ifeanyi-code]: https://github.com/iffyio/isolate
[ifeanyi-post]: http://ifeanyi.co/posts/linux-namespaces-part-4/
[mit]: https://opensource.org/licenses/MIT
[original]: https://lwn.net/Articles/531114/
