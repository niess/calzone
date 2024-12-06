#! /usr/bin/env python3
import calzone
from pathlib import Path

PREFIX = Path(__file__).parent

# =============================================================================
#
# Run the detector simulation.
#
# =============================================================================

simulation = calzone.Simulation(PREFIX / "geometry.toml")
simulation.physics.had_model = "FTFP_BERT" # enable hadronic interactions.
simulation.random.seed = 123456789 # for the reproducibility of this example.

scintillator = simulation.geometry.find("Scintillator")
scintillator.role = "record_deposits"

N = 1000
source = simulation.geometry.find("Source")
particles = simulation.particles(weight=True) \
    .pid("mu-")                               \
    .powerlaw(1E+02, 1E+05, exponent=-1)      \
    .on(source, direction="ingoing")          \
    .generate(N)

result = simulation.run(particles)

# =============================================================================
#
# Analyse the result.
#
# =============================================================================

def muon_flux(primaries):
    """Gaisser atmospheric muons flux model.

       Note that this model is for illustrative purpose only. In practice, one
       should also account for overburden water depth.
    """

    M = 105.66 # Muon rest mass, in MeV.
    GeV = 1E-03 # MeV to GeV.
    E = (primaries["energy"] + M) * GeV
    cos_theta = -primaries["direction"][:,2]

    K = 1.1 * E * cos_theta.clip(0.0, 1.0)

    return 0.14E-03 * E**-2.7 * (
        1.0 / (1.0 + K / 115.0) +
        0.054 / (1.0 + K / 850.0)
    ) # 1 / (MeV cm^2 sr s)


# Apply the muon flux model.
deposits = result.deposits[scintillator.path]
primaries = particles[deposits["event"]]
deposits["weight"] *= muon_flux(primaries) / N

# Select energy deposits between 0.1 and 3 MeV.
sel = (deposits["value"] > 0.1) & (deposits["value"] < 3.0)
rate = sum(deposits[sel]["weight"])

print(f"counting rate = {rate:.3E} Hz")
