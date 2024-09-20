Calzone
=======
*(CALorimeter ZONE)*

----

Calzone is a `Geant4`_ Python wrapper for simulating the energy deposition by
high-energy particles in a calorimeter. The interface has been designed with
simplicity in mind. Primary :py:func:`particles <calzone.particles>` are
:py:meth:`injected <calzone.Simulation.run>` into the simulation
:py:class:`volume <calzone.Geometry>` as a :external:py:class:`numpy.ndarray`,
and a :external:py:class:`numpy.ndarray` of energy deposits is returned. The
Monte Carlo :doc:`geometry <geometry>` is encoded in a Python
:external:py:class:`dict`, which can be loaded from configuration files, e.g.
using `JSON`_, `TOML`_ or `YAML`_ formats. This basic workflow is illustrated
below,

.. code:: python

   import calzone

   simulation = calzone.Simulation("geometry.toml")
   particles = calzone.particles(10000, energy=0.5, position=(0,0,1))
   deposits = simulation.run(particles)

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

.. ============================================================================
.. 
.. URL links.
.. 
.. ============================================================================

.. _JSON: https://www.json.org/json-en.html
.. _Geant4: https://geant4.web.cern.ch/docs/
.. _TOML: https://toml.io/en/
.. _YAML: https://yaml.org/
