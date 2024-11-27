# A benchmarking test case

This example implements a test case for the CPU benchmarking of gamma-rays Monte
Carlo transport.

A basic air-soil configuration is considered, comprising boxes. Gamma-rays are
generated in the air volume from the primary emission lines of radon descendants
and subsequently collected by a box-like detector positioned on the ground. The
corresponding geometry can be visualised as follows:

```bash
calzone display geometry.toml
```

The exercise consists in estimating the rate of photons collected by the
detector box, with secondary particles deactivated.
