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
simulation.geometry["Environment.Detector"].role = "catch_ingoing"
simulation.sample_particles = True
simulation.secondaries = False

N = 1000000
emission_lines = (
    # Pb-214 major emission lines.
    (0.242,  7.3),
    (0.295, 18.4),
    (0.352, 35.6),
    # Bi-214 major emission lines.
    (0.609, 45.5),
    (0.768,  4.9),
    (0.934,  3.1),
    (1.120, 14.9),
    (1.238,  5.8),
    (1.378,  4.0),
    (1.764, 15.3),
    (2.204,  4.9),
)
particles = simulation.particles(N) \
    .spectrum(emission_lines) \
    .inside("Environment") \
    .generate()

result = simulation.run(particles)

# =============================================================================
#
# Analyse the simulation results.
#
# =============================================================================

collected = result.particles["Environment.Detector"]
source_density = 1E-05 # Bq/cm^3
WORLD_SIZE, DETECTOR_WIDTH, DETECTOR_HEIGHT = 2E+05, 2E+03, 1E+03
source_volume = 0.5 * WORLD_SIZE**3 - DETECTOR_WIDTH**2 * DETECTOR_HEIGHT
total_activity = source_density * source_volume

efficiency = collected.size / N
sigma_efficiency = (efficiency * (1.0 - efficiency) / N)**0.5

rate = efficiency * total_activity * 1E-06 # MHz
sigma_rate = sigma_efficiency * total_activity * 1E-06 # MHz

print(f"rate = {rate:.2E} +- {sigma_rate:.2E} MHz")
print(f"efficiency = {efficiency:.1E} +- {sigma_efficiency:.1E}")
