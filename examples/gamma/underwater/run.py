#! /usr/bin/env python3
import calzone
from pathlib import Path

PREFIX = Path(__file__).parent

# =============================================================================
#
# Run the Monte Carlo simulation.
#
# =============================================================================

simulation = calzone.Simulation(PREFIX / "geometry.toml")
scintillator = simulation.geometry.find("Scintillator")
scintillator.role = "record_deposits"

N = 100000
source = simulation.geometry.find("Source")
particles = simulation.particles() \
    .pid("gamma")                  \
    .energy(1.0)                   \
    .inside(source)                \
    .generate(N)

result = simulation.run(particles)

# =============================================================================
#
# Analyse the simulation results.
#
# =============================================================================

n = result.deposits[scintillator.path].size
source_volume = source.volume()

litre = 1E-03 # cm3 to litre
veff = source_volume * n / N * litre
sigma_veff = source_volume * n**0.5 / N * litre

print(f"effective_volume = {veff:.1f} +- {sigma_veff:.1f} l")
