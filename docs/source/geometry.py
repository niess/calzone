#! /usr/bin/env python3
from matplotlib.patches import Rectangle
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
from PIL import Image

PREFIX = Path(__file__).parent

plt.style.use(PREFIX / "paper.mplstyle")


# =============================================================================
#
# Benchmark example.
#
# =============================================================================

img = np.asarray(Image.open(PREFIX / "include/benchmark-geometry.png"))
n, m, *_ = img.shape
ground_color = img[n // 3, m // 2] / 255.0

Environment = Rectangle(
    (-1, -1),
    2,
    2,
    ec="skyblue",
    fc="skyblue"
)

Ground = Rectangle(
    (-1, -1),
    2,
    1,
    ec="none",
    fc=ground_color
)

Detector = Rectangle(
    (-1E-02, 5E-05),
    2E-02,
    1E-02,
    ec="none",
    fc=(0.593957, 0.055367988, 0.636595)
)

plt.figure(figsize=(m / 100, n / 100))
plt.gca().add_patch(Environment)
plt.gca().add_patch(Ground)
plt.gca().add_patch(Detector)
plt.xticks(np.linspace(-1, 1, 5))
plt.yticks(np.linspace(-1, 1, 5))
plt.xlabel("x (km)")
plt.ylabel("z (km)")
plt.axis("equal")
plt.savefig(PREFIX / "include/benchmark-geometry.svg")


# =============================================================================
#
# Topography example.
#
# =============================================================================

img = np.asarray(Image.open(PREFIX / "include/topography-geometry.png"))
n, m, *_ = img.shape
ground_color = img[n // 2, m // 2] / 255.0

DEM_PADDING = 0.2
SKY_PADDING = 0.3

x = np.linspace(-500, 500, 1001)
r = np.absolute(x)
z = np.empty(x.shape)
sel = r < 202
z[sel] = 270 * np.exp(-(r[sel] / 250)**2)
sel = ~sel
z[sel] = 250 * np.exp(-r[sel] / 350)
Z_MIN = 100.0
z[z < Z_MIN] = Z_MIN
x *= 1E-03
z *= 1E-03

Environment = Rectangle(
    (x[0], min(z) - DEM_PADDING),
    x[-1] - x[0],
    max(z) + SKY_PADDING - (min(z) - DEM_PADDING),
    ec="skyblue",
    fc="skyblue"
)

plt.figure(figsize=(m / 100, n / 100))
plt.gca().add_patch(Environment)
plt.fill_between(x, min(z) - DEM_PADDING, z, color=ground_color)
plt.xlabel("x (km)")
plt.ylabel("z (km)")
plt.axis("equal")
plt.savefig(PREFIX / "include/topography-geometry.svg")


# =============================================================================
#
# Trajectograph example.
#
# =============================================================================

img = np.asarray(Image.open(PREFIX / "include/trajectograph-geometry.png"))
n, m, *_ = img.shape

layer_color = np.array((84, 73, 66)) / 255.0
lead_color = np.array((39, 39, 39)) / 255.0

chamber = [ 1.27, 36.0 ]
layers = [ -36, 0, 36 ]
planes = [ -90, -30, 30, 90 ]
lead = [ 10.0, 130.0 ]

plt.figure(figsize=(m / 100, n / 100))
for xp in planes:
    for zl in layers:
        xc, zc = chamber
        c = Rectangle(
            (xp - 0.5 * xc, zl - 0.5 * zc),
            xc,
            zc,
            ec="none",
            fc=layer_color,
        )
        plt.gca().add_patch(c)


xl, zl = lead
Lead = Rectangle(
    (-0.5 * xl, -0.5 * zl),
    xl,
    zl,
    ec="none",
    fc=lead_color,
)
plt.gca().add_patch(Lead)

plt.xlabel("y (cm)")
plt.ylabel("z (cm)")
plt.yticks(np.linspace(-50, 50, 3))
plt.axis("equal")
plt.savefig(PREFIX / "include/trajectograph-geometry.svg")

plt.show()
