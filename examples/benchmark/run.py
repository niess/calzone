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
detector = simulation.geometry.find("Detector")
detector.role = "catch_ingoing"
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
particles = simulation.particles() \
    .spectrum(emission_lines) \
    .inside("Environment") \
    .generate(N)

result = simulation.run(particles)

# =============================================================================
#
# Analyse the simulation results.
#
# =============================================================================

collected = result.particles[detector.path]
source_density = 1E-05 # Bq/cm^3
source_volume = simulation.geometry["Environment"].volume()
total_activity = source_density * source_volume

efficiency = collected.size / N
sigma_efficiency = (efficiency * (1.0 - efficiency) / N)**0.5

rate = efficiency * total_activity * 1E-06 # MHz
sigma_rate = sigma_efficiency * total_activity * 1E-06 # MHz

print(f"rate = {rate:.2E} +- {sigma_rate:.2E} MHz")
print(f"efficiency = {efficiency:.1E} +- {sigma_efficiency:.1E}")
