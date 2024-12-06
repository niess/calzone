#! /usr/bin/env python3
import calzone
import goupil
import numpy
from pathlib import Path

PREFIX = Path(__file__).parent

# =============================================================================
#
# Import the Monte Carlo geometry, using Calzone.
#
# =============================================================================

simulation = calzone.Simulation(PREFIX / "geometry.toml")

# =============================================================================
#
# Generate ingoing particles on the detector surface.
#
# =============================================================================

N = 1000000
particles = simulation.particles(weight=True)        \
    .pid("gamma")                                    \
    .on("Environment.Detector", direction="ingoing") \
    .generate(N)

# =============================================================================
#
# Run the backward simulation, using Goupil.
#
# See the goupil/examples/benchmark/backward.py example script for more in-depth
# explanations:
#
# https://github.com/niess/goupil/blob/master/examples/benchmark/backward.py
#
# =============================================================================

engine = goupil.TransportEngine(
    simulation.geometry.export() # This shares Calzone's geometry with Goupil.
)

engine.mode = "Backward"
engine.boundary = "Environment.Detector"

# Sample the energies of events entering the detector.
spectrum = numpy.array((
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
))
spectrum = goupil.DiscreteSpectrum(
    energies = spectrum[:,0], # MeV
    intensities = spectrum[:,1],
)

source_energies = spectrum.sample(particles, engine=engine)

# Slightly offset the ingoing particles (such that they start outside of the
# Detector volume, instead of inside).
EPSILON = 1E-03 # cm
particles["position"] -= EPSILON * particles["direction"]

# Run the backward simulation with Goupil.
status = engine.transport(particles, source_energies=source_energies)

# =============================================================================
#
# Analyse the simulation results.
#
# =============================================================================

sector = engine.geometry.locate(particles)
air_index = engine.geometry.sector_index("Environment")
selection = (status == goupil.TransportStatus.ENERGY_CONSTRAINT) & \
            (sector == air_index)
collected = particles[selection]

source_density = 1E-05 / (4.0 * numpy.pi)
rates = collected["weight"] * source_density / N * 1E-06 # MHz
rate = sum(rates)
sigma_rate = sum(rates**2 - (rate / N)**2)**0.5

efficiency = collected.size / N
sigma_efficiency = (efficiency * (1.0 - efficiency) / N)**0.5

print(f"rate = {rate:.2E} +- {sigma_rate:.2E} MHz")
print(f"efficiency = {efficiency:.1E} +- {sigma_efficiency:.1E}")
