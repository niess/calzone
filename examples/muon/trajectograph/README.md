# A muon trajectograph

This example showcases the simulation of a muon trajectograph, a device used in
[muography][MUOGRAPHY] applications. The detector is an assembly of [RPC][RPC]
chambers that are organised in sensitive layers and separated by a scatterer
layer, used to discriminate against low-energy particles. The geometry has been
modelled using only box shapes (see the [geometry.toml](geometry.toml),
[layer.toml](layer.toml) and [chamber.toml](chamber.toml) files). The result can
be visualised as

```bash
calzone display geometry.toml
```

Muons with an energy of 10 GeV are injected over the detector envelope, and the
resulting energy deposits in the [RPCs][RPC] gaps are printed, in the local
coordinates of the chamber.

Moreover, this example illustrates the reproducibility of the simulation by
replaying and displaying Monte Carlo tracks exclusively for events that result
in at least one energy deposit.


[MUOGRAPHY]: https://en.wikipedia.org/wiki/Muon_tomography
[RPC]: https://en.wikipedia.org/wiki/Resistive_plate_chamber
