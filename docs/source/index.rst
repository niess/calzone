CalZone
=======
*(CALorimeter ZONE)*

----

CalZone is a `Geant4`_ Python wrapper for simulating the energy depositions by
high energy particles in a calorimeter. The interface strives to be simple,
relying on :external:py:mod:`numpy`. That is, :py:func:`Primary
<calzone.primaries>` particles are :py:meth:`injected <calzone.Simulation.run>`
into the simulation :py:class:`volume <calzone.Geometry>` as a
:external:py:class:`numpy.ndarray`, and a :external:py:class:`numpy.ndarray` of
energy deposits is returned. The Monte Carlo :doc:`geometry <geometry>` is
encoded as a :external:py:class:`dict` structure, which can be defined through
configuration files, e.g. using `JSON`_ or `TOML`_ formats.

Documentation
-------------

.. toctree::
   :maxdepth: 2

   geometry
   api

System of units
---------------

.. note::

   CalZone uses the Centimetre-Gram-Second (CGS) system of units (e.g.
   g/cm\ :sup:`3` for a density), except for energies and momenta
   which are expressed in MeV and MeV/c respectively.

.. ============================================================================
.. 
.. URL links.
.. 
.. ============================================================================

.. _JSON: https://www.json.org/json-en.html
.. _Geant4: https://geant4.web.cern.ch/docs/
.. _TOML: https://toml.io/en/
