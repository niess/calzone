Calzone
=======
*(CALorimeter ZONE)*

----

Calzone is a Python package built on top of `Geant4`_. It was developed in the
context of geosciences with the objective of studying the emission of
radioactivity from volcanoes [TGV+20]_, and in particular to simulate the
response of gamma spectrometers deployed in the field. To this end, Calzone was
developed in conjunction with `Goupil`_ [NVT24]_, a backward gamma transport
engine, and is interoperable with the latter. Yet, both packages can be used
entirely independently, if necessary.

Calzone's interface has been designed with simplicity in mind. Source particles
are injected into the simulation volume as a :external:py:class:`numpy.ndarray`,
and a :external:py:class:`numpy.ndarray` of collected energy deposits (or
particles) is returned. The Monte Carlo :doc:`geometry <geometry>` is encoded in
a Python :external:py:class:`dict`, which can be loaded from configuration
files, e.g. using `JSON`_, `TOML`_ or `YAML`_ formats. This basic workflow is
illustrated below,

.. code:: python

   import calzone

   simulation = calzone.Simulation("geometry.toml")
   particles = calzone.particles(
       10000,
       pid="e-",
       energy=0.5,       # MeV
       position=(0,0,1)  # cm
   )
   deposits = simulation.run(particles).deposits

Calzone encourages the use of meshes to describe the Monte Carlo geometry.
Various mesh formats are supported, such as `OBJ`_, `STL`_, `GeoTIFF`_ and
`Turtle`_ / `PNG`_ [NBCM20]_. These formats can be used to encode the
components of a detector (exported from a `CAD`_ scheme) or a Digital Elevation
Model (`DEM`_) describing the surrounding terrain. Additionally, Calzone
features an :doc:`interactive display <display>` that allows users to navigate
through the Monte Carlo geometry and to inspect Monte Carlo tracks.


System of units
---------------

.. note::

   Calzone uses the Centimetre-Gram-Second (CGS) system of units (e.g.
   g/cm\ :sup:`3` for a density), except for energies and momenta
   which are expressed in MeV and MeV/c respectively.

Documentation
-------------

.. toctree::
   :maxdepth: 2

   installation
   geometry
   api
   display
   references

.. ============================================================================
.. 
.. URL links.
.. 
.. ============================================================================

.. _CAD: https://en.wikipedia.org/wiki/Computer-aided_design
.. _DEM: https://en.wikipedia.org/wiki/Digital_elevation_model
.. _JSON: https://www.json.org/json-en.html
.. _Geant4: https://geant4.web.cern.ch/docs/
.. _GeoTIFF: https://fr.wikipedia.org/wiki/GeoTIFF
.. _Goupil: https://github.com/niess/goupil
.. _OBJ: https://en.wikipedia.org/wiki/Wavefront_.obj_file
.. _PNG: https://en.wikipedia.org/wiki/PNG
.. _STL: https://en.wikipedia.org/wiki/STL_(file_format)
.. _TOML: https://toml.io/en/
.. _Turtle: https://github.com/niess/turtle
.. _YAML: https://yaml.org/
