Geometry definition
===================

A `Geant4`_ Monte Carlo geometry consists of a hierarchy of nested
`G4VPhysicalVolume`_\ s, starting from a single root (world) volume. This
structure can be represented using only basic Python objects
(:external:py:class:`dict`, :external:py:class:`float`,
:external:py:class:`list`, etc.) that have native representations in common
configuration languages, such as `JSON`_ or `TOML`_. This approach allows us to
conveniently define a Monte Carlo geometry from a file, according to which the
actual Geant4 geometry is built.


Volume definition
-----------------

A geometry volume is represented by a Python :external:py:class:`dict` item,
where the item key is the volume name, and the item value is another
:external:py:class:`dict` object containing the volume properties. For instance,

>>> root = { "Environment": { "material" : "G4_AIR", ... }}

Semantic
~~~~~~~~

Volume names must be capitalized, and they must contain only alpha-numeric
characters. Thus, volume names are typically `CamelCased`. On the contrary,
volume properties follow the `snake_case` semantic.

Pathname
~~~~~~~~

A geometry volume is uniquely identified by its absolute pathname, formed by the
dot-junction of all its ancester volumes. For instance, the
:python:`"Environment.Detector"` pathname refers to the :python:`"Detector"`
volume located inside the :python:`"Environment"` one.

.. note::

   The :external:py:class:`dict` representation of the geometry ensures that
   sister volumes (i.e. direct daughters of the same mother volume) have
   distinct names. As a result, pathnames are unique, within a same geometry.

Properties
~~~~~~~~~~

The properties of a Monte Carlo volume are summarised in :numref:`volume
properties` below. It is required to define at least a :python:`"material"` and
a :python:`shape`. For instance, the minimal properties of a box volume would be
as,

>>> properties = { "material": "G4_AIR", "box": { "size": [ 1.0, 1.0, 1.0 ] }}

Note that only a single shape can be defined per volume. See the section related
to :ref:`shapes <geometry:Shape definitions>` for the corresponding properties.

.. _volume properties:

.. list-table:: Summary of volume properties.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Type
     - Default
   * - :python:`"material"`
     - :python:`str`
     - 
   * - :python:`shape` (:python:`"box"`, :python:`"cylinder"`,
       :python:`"envelope"`, :python:`"sphere"` or :python:`"tessellation"`)
     - :python:`dict`
     - 
   * - :python:`"position"`
     - :python:`[float; 3]`
     - :python:`numpy.zeros(3)`
   * - :python:`"rotation"`
     - :python:`[[float; 3]; 3]`
     - :python:`numpy.eye(3)`
   * - :python:`"sensitive"`
     - :python:`bool`
     - :python:`False`
   * - :python:`"subtract"`
     - :python:`str`
     - :python:`None`
   * - :python:`"overlaps"`
     - :python:`dict`
     - :python:`None`
   * - :python:`DaughterName`
     - :python:`dict`
     - :python:`None`

.. topic:: Positioning properties.

   The optional :python:`"position"` and :python:`"rotation"` volumes are
   relative to the mother volume frame. By default, the volume is placed
   unrotated with its origin coinciding with the mother one.

.. topic:: Sensitive volumes.

   The :python:`"sensitive"` flag determines whether a volume records energy
   deposits or not. By default, Monte Carlo volumes are inert.

.. topic:: Daughter volumes.

   Daughter volumes are included directly asides the volume properties. They are
   identified by they CamelCase semantic.

Overlaps
~~~~~~~~

The :python:`"subtract"` and :python:`"overlaps"` volume properties let us
handle overlaping sister volumes in two different ways. The :python:`"subtract"`
property explicitly specifies a sister volume (by its name) whose shape must be
subtracted from the volume one. The typical use case is to sutract the
:python:`"Detector"` volume from the :python:`"Ground"` one, when the latter is
not smooth.

The :python:`"overlaps"` property let us specify pairs of sister volumes, as
(key, value), that overlap erroneously, e.g. due to numeric approximations. For
instance,

>>> { "overlaps": { "Bottom": [ "Left", "Right" ], "Top": "Left" }}

These volumes are patched using an iterative subtraction procedure. Note that
this procedure provides no guarantees on which volume is subtracted or not. It
is expected to be used only for patching small (erroneous) overlaps.

Shape definitions
-----------------

Box shape
~~~~~~~~~

.. list-table:: Box shape properties.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Type
     - Default
   * - :python:`"size"`
     - :python:`[float; 3]`
     - 

Cylinder shape
~~~~~~~~~~~~~~

.. list-table:: Cylinder shape properties.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Type
     - Default
   * - :python:`"length"`
     - :python:`float`
     - 
   * - :python:`"radius"`
     - :python:`float`
     - 

Envelope shape
~~~~~~~~~~~~~~

.. list-table:: Envelope shape properties.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Type
     - Default
   * - :python:`"safety"`
     - :python:`float`
     - :python:`0.01`
   * - :python:`"shape"`
     - :python:`str`
     - :python:`"box"`

Sphere shape
~~~~~~~~~~~~

.. list-table:: Cylinder shape properties.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Type
     - Default
   * - :python:`"radius"`
     - :python:`float`
     - 

Tessellation shape
~~~~~~~~~~~~~~~~~~

.. list-table:: Tessellation shape properties.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Type
     - Default
   * - :python:`"path"`
     - :python:`str`
     - 
   * - :python:`"units"`
     - :python:`str`
     - :python:`"cm"`

.. list-table:: Extra topography properties.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Type
     - Default
   * - :python:`"min_depth"`
     - :python:`float`
     - 100.0 (in map units)
   * - :python:`"origin"`
     - :python:`[float; 3]`
     - :python:`numpy.zeros(3)`
   * - :python:`"regular"`
     - :python:`bool`
     - :python:`False`

Material definition
-------------------

.. tip::

   A collection of standard atomic elements and materials is readily available
   from the Geant4 `NIST`_ database. For example, :python:`"G4_Na"`,
   :python:`"G4_AIR"`, etc. Depending on your application, you may not need to
   define your own materials.

Atomic elements
~~~~~~~~~~~~~~~

Molecules
~~~~~~~~~

Mixtures
~~~~~~~~

.. ============================================================================
.. 
.. URL links.
.. 
.. ============================================================================

.. _JSON: https://www.json.org/json-en.html
.. _G4Material: https://geant4.kek.jp/Reference/11.2.0/classG4Material.html
.. _G4VPhysicalVolume: https://geant4.kek.jp/Reference/11.2.0/classG4VPhysicalVolume.html
.. _G4VSolid: https://geant4.kek.jp/Reference/11.2.0/classG4VSolid.html
.. _Geant4: https://geant4.web.cern.ch/docs/
.. _NIST: https://geant4-userdoc.web.cern.ch/UsersGuides/ForApplicationDeveloper/html/Appendix/materialNames.html?highlight=nist#
.. _TOML: https://toml.io/en/
