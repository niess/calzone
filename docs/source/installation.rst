Installing Calzone
==================

.. topic:: Python version

   Calzone requires CPython **3.7** or higher in order to operate.


From PyPI
---------

Binary distributions of Calzone are available from `PyPI`_, for Linux (x86_64),
as

.. code:: bash

   pip3 install calzone

In addition, you might need to install some `optional dependencies`_ in order to
import geometries (which can be done using :bash:`pip3 install`, as well).


From source
-----------

Calzone source is available from `GitHub`_, e.g. as

.. code:: bash

   git clone https://github.com/niess/calzone

To build Calzone from source, you will require the `Rust toolchain`_ and have
`Geant4`_ pre-installed with C++17 and GDML enabled, but with multithreading
disabled. Once the Geant4 environment has been set up, Calzone will be built as
a Rust shared library. For example, on Linux, the following commands builds the
Calzone package in-source (under `src/python/calzone
<https://github.com/niess/calzone/tree/master/src/python/calzone>`_).

.. code:: bash

   # Initialise the Geant4 environment.
   source $GEANT4_PREFIX/bin/geant.sh

   # Build the Calzone binary.
   cargo build --release

   # Link the resulting binary.
   ln -rs target/release/libcalzone.so src/python/calzone/calzone.so


Optional dependencies
---------------------

Danton might require some optional dependencies to be installed, depending on
your desired format for encoding geometries, and on your Python version. These
are listed in :numref:`tab-optional-dependencies` below.

.. _tab-optional-dependencies:

.. list-table:: Optional dependencies.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Geometry format
     - Python version
     - Required package
   * - `TOML`_
     - 3.10, or less
     - `tomli`_
   * - `YAML`_
     - 3.7, or more
     - `PyYAML`_


.. ============================================================================
.. 
.. URL links.
.. 
.. ============================================================================

.. _Geant4: https://geant4.web.cern.ch/docs/
.. _GitHub: https://github.com/niess/calzone
.. _PyPI: https://pypi.org/project/calzone/
.. _PyYAML: https://pypi.org/project/PyYAML/
.. _Rust toolchain: https://www.rust-lang.org/tools/install
.. _TOML: https://toml.io/en/
.. _tomli: https://pypi.org/project/tomli/
.. _YAML: https://yaml.org/
