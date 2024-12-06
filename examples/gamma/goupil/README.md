# An example of using Calzone and Goupil together

This example demonstrates the implementation of the [CPU benchmarking test
case](../benchmark) using Calzone for the geometry, but [Goupil][GOUPIL] for the
transport of gamma-rays, which is performed backwards.

For further details on the backward procedure, please refer to [(niess et al.,
2024)][NIESS_2024], or to the corresponding Goupil
[example](https://github.com/niess/goupil/blob/master/examples/benchmark/backward.py).

> [!NOTE]
>
> Although this is not the case in the example, for a comprehensive computation,
> the particles generated on the detector surface would be further forward
> simulated with Calzone.

[GOUPIL]: https://github.com/niess/goupil
[NIESS_2024]: https://arxiv.org/abs/2412.02414
