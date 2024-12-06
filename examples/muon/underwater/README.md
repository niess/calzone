# Muons background in an underwater gamma-ray spectrometer

This example showcases the simulation of the atmospheric muons background
induced in an underwater gamma-ray spectrometer, which has been modelled using
only cylinder shapes (see the [detector.toml](detector.toml) file). The
corresponding geometry can be visualised as

```bash
calzone display detector.toml
```

Atmospheric muons are injected over a sphere of radius 1m centred on the
spectrometer. The resulting counting rate is estimated by Monte Carlo
simulation, where the majority of recorded hits are actually secondary gamma
rays (radiated by ionised electrons).

> [!NOTE]
>
> The counting rate computation is based on the assumption that a muon flux
> model is provided for the detector location. For the purpose of this example,
> a simplified model is used, which does not take into account overburden water
> depth.
