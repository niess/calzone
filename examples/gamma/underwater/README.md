# An underwater gamma-ray spectrometer

This example showcases the simulation of an underwater gamma-ray spectrometer,
which has been modelled using only cylinder shapes (see the
[detector.toml](detector.toml) file). The corresponding geometry can be
visualised as

```bash
calzone display detector.toml
```

The gamma-ray sources are assumed to be uniformly distributed over the water
volume, with an emission line set to 1 MeV. The corresponding effective volume
is estimated by Monte Carlo simulation.

To enhance the efficiency of the simulation, the generation of gamma rays is
confined to a radius of 1m sphere around the detector. However, the world volume
is larger to accommodate trajectories that escape from the generation sphere but
ultimately reach the detector.
