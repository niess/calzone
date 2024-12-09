#! /usr/bin/env python3
import calzone
import numpy
from pathlib import Path
import re

PREFIX = Path(__file__).parent

# =============================================================================
#
# Run the detector simulation.
#
# =============================================================================

simulation = calzone.Simulation(PREFIX / "geometry.toml")
simulation.sample_deposits = "detailed"
simulation.random.seed = 123456789 # for the reproducibility of this example.

N = 100
particles = simulation.particles(weight=True)           \
    .pid("mu-")                                         \
    .energy(1E+04)                                      \
    .on(simulation.geometry.root, direction="ingoing")  \
    .generate(N)

result = simulation.run(particles)

# =============================================================================
#
# Print the ionising energy deposits (`line`), in chamber's local coordinates.
#
# =============================================================================

def format_point(p):
    return f"{p[0]:7.3f} {p[1]:7.3f} {p[2]:7.3f}"

pattern = re.compile("Layer([0-9]+)[.]Chamber([0-9]+)")

for (layer, deposits) in result.deposits.items():
    # Compute the local coordinates of line-deposits segments.
    volume = simulation.geometry[layer]
    start = volume.local_coordinates(deposits.line["start"])
    end = volume.local_coordinates(deposits.line["end"])

    # Parse the layer and chamber indices.
    match = pattern.search(layer)
    i, j, k = match.group(1), match.group(2)[0], match.group(2)[1]
    print(f"- Layer {i}, Chamber ({j}, {k}):")

    # Print the line deposits.
    for (i, vi) in enumerate(deposits.line["value"]):
        si = format_point(start[i,:])
        ei = format_point(end[i,:])
        print(f"  {vi:.3E}   {si}   {ei}")


# =============================================================================
#
# Display events with at least one line deposit.
#
# =============================================================================

# Find events with at least one deposits, and map their corresponding random
# engine state.
random_indices = {}
for deposits in result.deposits.values():
    for deposit in deposits.line:
        random_indices[deposit["event"]] = deposit["random_index"]
events = numpy.array([event for event in random_indices.keys()])
random_indices = numpy.array([index for index in random_indices.values()])

# Re-simulate the selected events, but enabling tracking this time.
simulation.tracking = True
result = simulation.run(particles[events], random_indices=random_indices)

# Display the result (using the `calzone-display` extension).
simulation.geometry.root.display(result)
