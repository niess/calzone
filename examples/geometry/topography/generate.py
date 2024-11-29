#! /usr/bin/env python3
import calzone
import numpy as np
from pathlib import Path


PREFIX = Path(__file__).parent


# =============================================================================
#
# Generate topography data using the Showa-Shinzan analytical model (Nishiyama
# et al., Geophys. J. Int. (2016)), which is employed here for illustrative
# purposes. The resulting Digital Elevation Model (DEM) is returned as a
# calzone.Map object.
#
# Note that we could have instead loaded an existing DEM (in Turtle/PNG or
# GeoTIFF) format, instead of generating one.
#
# =============================================================================

def generate_dem():
    """Generate a Digital Elevation Model."""

    x = np.linspace(-500, 500, 1001)
    y = np.linspace(-600, 600, 1201)
    X, Y = np.meshgrid(x, y)
    z = np.empty(X.shape)
    R = np.sqrt(X**2 + Y**2)
    sel = R < 202
    z[sel] = 270 * np.exp(-(R[sel] / 250)**2)
    sel = ~sel
    z[sel] = 250 * np.exp(-R[sel] / 350)

    return calzone.Map.from_array(z, (x[0], x[-1]), (y[0], y[-1]))


topography = generate_dem()


# =============================================================================
#
# Modify the DEM.
#
# Let us clamp the elevation values below 100m, for illustrative purposes,
# again.
#
# =============================================================================

Z_MIN = 100.0
sel = topography.z < Z_MIN
topography.z[sel] = Z_MIN


# =============================================================================
#
# Export the resulting DEM for calzone.
#
# The model is exported in Turtle/PNG format, which is supported by Calzone
# geometries. Alternatively, an external tool (e.g. GDAL) could be used to
# export the elevation data in GeoTIFF format (which is also supported by
# Calzone geometries).
#
# =============================================================================

path = PREFIX / "meshes/terrain.png"
path.parent.mkdir(exist_ok=True)
topography.dump(path)
