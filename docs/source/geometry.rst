Geometry definition
===================

A `Geant4`_ Monte Carlo geometry consists of a hierarchy of nested
`G4VPhysicalVolume`_\ s, starting from a single root ("world") volume. CalZone
represents this structure using base Python objects
(:external:py:class:`bool`, :external:py:class:`dict`,
:external:py:class:`float`, :external:py:class:`int`, :external:py:class:`list`
and :external:py:class:`str`) that have associated representations in common
configuration languages, such as `JSON`_ or `TOML`_.

Geometry objects
----------------

All geometry objects (volumes, shapes, materials, etc.) adhere to the same
data structure. A geometry object is represented by a Python
:external:py:class:`dict` item (i.e. a :python:`[key: str, value: dict]`
pair), where the :python:`key` is the object name, and the :python:`value` might
be another :external:py:class:`dict`, e.g. containing the object's properties.
To illustrate, a cylinder shape writes,

>>> { "cylinder": { "length": 2.0, "radius": 1.0 }}

.. topic:: Objects names

   Proper names (i.e. designating specific volumes, materials, etc.) must be
   capitalised and comprise solely alpha-numeric characters. Thus, proper names
   are typically `CamelCased`. Conversely, common names (designating properties,
   shape types, etc.) adhere to the `snake_case` syntax.

Geometry structure
------------------

A geometry definition starts with a root volume, for instance as follows,

>>> { "Environment": { "cylinder" : { "length": 2.0, "radius": 1.0 }, ... }}

There can be only one root volume in a geometry. However, the geometry
:external:py:class:`dict` might contain an additional :python:`"materials"` key,
for describing the geometry materials. The corresponding structure is summarised
below, in :numref:`tab-geometry-items`.

.. _tab-geometry-items:

.. list-table:: Geometry items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`RootVolumeName`
     - :python:`dict` (:numref:`tab-volume-items`)
     - 
   * - :python:`"materials"`
     - :python:`dict` (:numref:`tab-materials-items`)
     - :python:`None`

.. topic:: Pathname

   Geometry volumes are identified by their absolute pathname, which is formed
   by the dot-jointure of their own name with all of their ancestors names. For
   example, the :python:`"Environment.Detector"` pathname refers to the
   :python:`"Detector"` volume located inside the :python:`"Environment"`
   volume.

   The :external:py:class:`dict` representation of the geometry ensures that
   pathnames are unique within a given geometry.

Volume definition
-----------------

The items of a Monte Carlo volume are presented in :numref:`tab-volume-items`
below. Note that it is mandatory to define a *material* and a *shape*. For
example, a box volume would be represented as follows,

>>> { "material": "G4_AIR", "box": { "size": [ 1.0, 1.0, 1.0 ] }}

Note also that a volume can only have a single shape item (but multiple daughter
volumes). For further information on shape types and their corresponding items,
see :ref:`geometry:Shape definition`.

.. _tab-volume-items:

.. list-table:: Volume items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"material"`
     - :python:`str`
     - 
   * - :python:`shape_type`
     - :python:`dict` (:ref:`Shape items <geometry:Shape definition>`)
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
     - :python:`dict` (:numref:`tab-overlaps-items`)
     - :python:`None`
   * - :python:`DaughterName`
     - :python:`dict` (:numref:`tab-volume-items`)
     - :python:`None`

.. topic:: Positioning properties.

   The optional :python:`"position"` and :python:`"rotation"` properties are
   relative to the mother volume frame. By default, the volume is placed
   unrotated with its origin coinciding with the mother one.

.. topic:: Sensitive volumes.

   The :python:`"sensitive"` flag determines whether a volume records energy
   deposits or not. By default, Monte Carlo volumes are inert.

.. topic:: Daughter volumes.

   The daughter volumes are included directly with the volume properties. They
   are identified by their `CamelCase` syntax.

Overlaps
~~~~~~~~

The :python:`"subtract"` and :python:`"overlaps"` volume properties address the
issue of overlaps between sister volumes in two distinct ways. The
:python:`"subtract"` property explicitly specifies a sister volume (by its name)
whose shape is to be subtracted from the current volume. This can be employed,
for instance, to subtract a portion of a :python:`"Ground"` volume to
accommodate a partially buried :python:`"Detector"` volume.

.. note::

   Only unsubtracted volumes can be subtracted from. Consequently, the
   *subtract* property does not permit the formation of subtraction chains.

The :python:`"overlaps"` property indicates pairs of overlapping daughter
volumes, (see :numref:`tab-overlaps-items`), for instance as,

>>> { "overlaps": { "Bottom": [ "Left", "Right" ], "Top": "Left" }}

These volumes are separated using an iterative subtraction procedure. It should
be noted that this procedure does not guarantee which volume is subtracted or
not. It is therefore recommended that this method be used only for the purpose
of patching small (erroneous) overlaps (e.g. due to numeric approximations).

.. _tab-overlaps-items:

.. list-table:: Overlaps items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`VolumeName`
     - :python:`str | [str]`
     - 

Shape definition
----------------

The available shape types are described below. Note that the shape type name
follows the `snake_case` syntax (i.e. like property names).

Box shape
~~~~~~~~~

An axis-aligned box (`G4Box`_), centred on the origin, and defined by its *size*
(in cm) along the x, y and z-axis.

.. list-table:: Box items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"size"`
     - :python:`[float; 3]`
     - 

Cylinder shape
~~~~~~~~~~~~~~

A cylinder of revolution around the z-axis (`G4Tubs`_), centred on the origin,
and defined by its *length* (in cm) along the z-axis and its *radius* (in cm) in
the xOy plane.

.. list-table:: Cylinder items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"length"`
     - :python:`float`
     - 
   * - :python:`"radius"`
     - :python:`float`
     - 

Envelope shape
~~~~~~~~~~~~~~

A bounding envelope with a specified *shape*, whose size is determined by the
bounded daughter volumes. The *safety* parameter (in cm) allows for extra space
around bounded objects.

.. list-table:: Envelope items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"safety"`
     - :python:`float`
     - :python:`0.01` (cm)
   * - :python:`"shape"`
     - :python:`str`
     - :python:`"box"`

Sphere shape
~~~~~~~~~~~~

A sphere (`G4Orb`_), centred on the origin, and defined by its *radius* (in cm).

.. list-table:: Sphere items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"radius"`
     - :python:`float`
     - 

Tessellation shape
~~~~~~~~~~~~~~~~~~

A 3D tessellation defined from a data file (*path* property) with the specified
length *units*.

.. list-table:: Tessellation items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"path"`
     - :python:`str`
     - 
   * - :python:`"units"`
     - :python:`str`
     - :python:`"cm"`

The actual shape depends on the data file format. If the file is a 3D `STL`_
model, then the model is directly imported. Alternatively, the data can also be
a surface described by a Digital Elevation Model (`DEM`_). In this case,
elevation values are assumed to be along the z-axis, and the surface is closed
by adding side and bottom faces. The additional properties described in
:numref:`tab-topography-items` control the generated 3D shape.

.. tip::

   The :py:meth:`Map.dump() <calzone.Map.dump>` method allows one to export the
   generated 3D shape in `STL`_ format.

.. _tab-topography-items:

.. list-table:: DEM tesselation items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"extra_depth"`
     - :python:`float`
     - 100.0 (in map units)
   * - :python:`"origin"`
     - :python:`[float; 3]`
     - :python:`numpy.zeros(3)`
   * - :python:`"regular"`
     - :python:`bool`
     - :python:`False`

.. topic:: Geometric properties.

   The *origin* property defines the origin of the 3D shape in the DEM
   coordinates system. The *extra_depth* property extends the shape
   below the DEM's minimum elevation value.

.. topic:: Meshing type.

   The *regular* flag controls the meshing. By default, a non-regular -but
   optimised- meshing is used. However, this is not supported by the Geant4
   traversal :py:attr:`algorithm <calzone.GeometryBuilder.algorithm>`.
   Therefore, a *regular* meshing must be selected when using the latter
   algorithm.

Materials definition
--------------------

A Geant4 material (`G4Material`_) can be defined either as an assembly of atomic
elements (`G4Element`_\ s), denoted :ref:`Molecule <geometry:Molecules>` herein,
or as a :ref:`Mixture <geometry:Mixtures>` of other materials.

.. tip::

   A collection of standard atomic elements and materials is readily available
   from the Geant4 `NIST`_ database. For example, :python:`"G4_H"`,
   :python:`"G4_AIR"`, etc. Depending on your application, you may not need to
   define your own materials.

Materials table
~~~~~~~~~~~~~~~

The structure of a materials table is described by :numref:`tab-materials-items`
(et al.) below. :ref:`geometry:Molecules` and :ref:`geometry:Mixtures` are
explictily separated. In addition, the materials table may also contain (custom)
atomic elements. For instance,

>>> { "molecules": { "H2O": { ... }}, "mixtures": { "Air": { ... }}}

.. _tab-materials-items:

.. list-table:: Materials items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"elements"`
     - :python:`dict` (:numref:`tab-elements-items`)
     - :python:`None`
   * - :python:`"molecules"`
     - :python:`dict` (:numref:`tab-molecules-items`)
     - :python:`None`
   * - :python:`"mixtures"`
     - :python:`dict` (:numref:`tab-mixtures-items`)
     - :python:`None`

.. _tab-elements-items:

.. list-table:: Atomic elements items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`ElementName`
     - :python:`dict` (:numref:`tab-element-items`)
     - 

.. _tab-molecules-items:

.. list-table:: Molecules items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`MoleculeName`
     - :python:`dict` (:numref:`tab-molecule-items`)
     - 

.. _tab-mixtures-items:

.. list-table:: Mixtures items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`MixtureName`
     - :python:`dict` (:numref:`tab-mixture-items`)
     - 

Atomic elements
~~~~~~~~~~~~~~~

Atomic elements are specified by their atomic number (*Z*) and by their mass
number (*A*, in g/mol). Optionally, a *symbol* can be specified.

.. _tab-element-items:

.. list-table:: Atomic element items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"Z"`
     - :python:`float`
     - 
   * - :python:`"A"`
     - :python:`float`
     - 
   * - :python:`"symbol"`
     - :python:`str`
     - :python:`None`

Molecules
~~~~~~~~~

Molecules are specified by their *density* (in g/cm\ :sup:`3`) and their
composition (in atomic elements). Optionaly, a *state* can be specified (
:python:`"gas"`, :python:`"liquid"` or :python:`"solid"`). For instance,

>>> { "H": 2, "O": 1, "density": 1.0, "state": "liquid" }

.. _tab-molecule-items:

.. list-table:: Molecule items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"density"`
     - :python:`float`
     - 
   * - :python:`"state"`
     - :python:`str`
     - :python:`None`
   * - :python:`ElementName`
     - :python:`int`
     - 

Mixtures
~~~~~~~~

Mixtures are specified by their *density* (in g/cm\ :sup:`3`) and their **mass**
composition. Optionaly, a *state* can be specified ( :python:`"gas"`,
:python:`"liquid"` or :python:`"solid"`). For instance,

>>> { "N": 0.76, "O": 0.23, "Ar": 0.01, "density": 1.205E-03, "state": "gas" }

.. _tab-mixture-items:

.. list-table:: Mixture items.
   :width: 75%
   :widths: auto
   :header-rows: 1

   * - Key
     - Value type
     - Default value
   * - :python:`"density"`
     - :python:`float`
     - 
   * - :python:`"state"`
     - :python:`str`
     - :python:`None`
   * - :python:`MaterialName`
     - :python:`float`
     - 

.. ============================================================================
.. 
.. URL links.
.. 
.. ============================================================================

.. _DEM: https://en.wikipedia.org/wiki/Digital_elevation_model
.. _JSON: https://www.json.org/json-en.html
.. _G4Box: https://geant4.kek.jp/Reference/11.2.0/classG4Box.html
.. _G4Element: https://geant4.kek.jp/Reference/11.2.0/classG4Element.html
.. _G4Material: https://geant4.kek.jp/Reference/11.2.0/classG4Material.html
.. _G4Orb: https://geant4.kek.jp/Reference/11.2.0/classG4Orb.html
.. _G4Tubs: https://geant4.kek.jp/Reference/11.2.0/classG4Tubs.html
.. _G4VPhysicalVolume: https://geant4.kek.jp/Reference/11.2.0/classG4VPhysicalVolume.html
.. _G4VSolid: https://geant4.kek.jp/Reference/11.2.0/classG4VSolid.html
.. _Geant4: https://geant4.web.cern.ch/docs/
.. _NIST: https://geant4-userdoc.web.cern.ch/UsersGuides/ForApplicationDeveloper/html/Appendix/materialNames.html?highlight=nist#
.. _STL: https://en.wikipedia.org/wiki/STL_(file_format)
.. _TOML: https://toml.io/en/
