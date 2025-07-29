Installing Calzone
==================

.. topic:: Python version

   Calzone requires Python **3.7** or higher.

.. topic:: Interactive display

   Calzone's interactive display is distributed as an optional extension
   package.


From PyPI
---------

Binary distributions of Calzone are available from `PyPI`_, e.g. as

.. code:: bash

   pip3 install calzone

or alternatively

.. code:: bash

   pip3 install calzone-display

in order to install both Calzone and its interactive display.

In addition, you might need to install some `optional dependencies`_ in order to
import geometries (which can be done using :bash:`pip3 install`, as well).


Geant4 data
-----------

In order to operate, Calzone requires 2 GB of Geant4 data tables, which are not
included in the Python package. Once Calzone has been installed (or updated),
these data can be downloaded (or updated) as

.. code:: bash

   python3 -m calzone download

Alternatively, the :bash:`G4_DATA_DIR` environment variable can be set to the
location of already existing Geant4 data, e.g. from another Geant4 installation.


From source
-----------

Calzone source is available from `GitHub`_, e.g. as

.. code:: bash

   git clone --recursive https://github.com/niess/calzone

To build Calzone from source, you will require the `Rust toolchain`_ and have
`Geant4`_ pre-installed with C++17 enabled, but with multithreading disabled.
Once the Geant4 environment has been set up, Calzone will be built as a Rust
shared library. For example, on Linux, the following commands builds the Calzone
package in-source (under `src/python/calzone
<https://github.com/niess/calzone/tree/master/src/python/calzone>`_).

.. code:: bash

   # Initialise the Geant4 environment.
   source $GEANT4_PREFIX/bin/geant.sh

   # Build the Calzone binary.
   cargo build --release

   # Link the resulting binary.
   ln -rs target/release/libcalzone.so src/python/calzone/calzone.so

The source of the interactive display is also available from `GitHub
<GitHub-Display_>`_, e.g. as

.. code:: bash

   git clone https://github.com/niess/calzone-display

The display package is built on top of the `Bevy Engine <BevyEngine_>`_. Please
refer to the corresponding `Setup`_ documentation for build time dependencies
and for possible optimisations.


Optional dependencies
---------------------

Calzone might require some optional dependencies to be installed, depending on
your desired format for encoding maps and geometries, and on your Python
version. These are listed in :numref:`tab-optional-dependencies` below.

.. _tab-optional-dependencies:

.. list-table:: Optional dependencies.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Format
     - Python version
     - Required package
   * - `GeoTIFF`_
     - 3.7, or more
     - `geotiff <PyGeotiff_>`_
   * - `PNG`_
     - 3.7, or more
     - `Pillow`_
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

.. _BevyEngine: https://bevyengine.org/
.. _Geant4: https://geant4.web.cern.ch/docs/
.. _GeoTIFF: https://en.wikipedia.org/wiki/GeoTIFF
.. _PyGeotiff: https://github.com/KipCrossing/geotiff
.. _GitHub: https://github.com/niess/calzone
.. _GitHub-Display: https://github.com/niess/calzone-display
.. _Pillow: https://python-pillow.org/
.. _PNG: https://en.wikipedia.org/wiki/PNG
.. _PyPI: https://pypi.org/project/calzone/
.. _PyYAML: https://pypi.org/project/PyYAML/
.. _Rust toolchain: https://www.rust-lang.org/tools/install
.. _Setup: https://bevyengine.org/learn/quick-start/getting-started/setup/
.. _TOML: https://toml.io/en/
.. _tomli: https://pypi.org/project/tomli/
.. _YAML: https://yaml.org/
