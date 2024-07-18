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
simulation.geometry["Environment.Source.Detector"].role = "record_deposits"

N = 100000
particles = simulation.particles(N) \
    .energy(1.0) \
    .inside("Environment.Source") \
    .generate()

deposits = simulation.run(particles)

# =============================================================================
#
# Analyse the simulation results.
#
# =============================================================================

n = deposits["Environment.Source.Detector"].size
source_volume = simulation.geometry["Environment.Source"].volume()

litre = 1E-03 # cm3 to litre
veff = source_volume * n / N * litre
sigma_veff = source_volume * n**0.5 / N * litre

print(f"effective_volume = {veff:.1f} +- {sigma_veff:.1f} l")
