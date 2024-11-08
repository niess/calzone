Python interface
================


.. autofunction:: calzone.define

   The material(s) *definition* can be provided directly as a Python
   :python:`dict` object, or loaded from a *definition* file (in Gate DB, JSON
   or TOML format). For instance, the following defines materials from a Gate DB
   file.

   >>> calzone.define("materials.db")

   See the :ref:`Materials definition <geometry:Materials definition>` section
   for further information.

   .. important::

      Materials are uniquely identified by their name. However, once loaded to
      Geant4, a material cannot be unloaded, nor modified. If a different
      definition is provided for an existing (already loaded) material, then a
      :external:py:class:`ValueError` is raised.

----

.. autofunction:: calzone.download

   `Geant4`_  requires 2 |nbsp| GB of materials data in order to operate. These
   data are not distributed with Calzone, but are available for download from
   the `Geant4`_ website. This method automates the process of downloading these
   data.

   The *destination* argument specifies where the downloaded data should be
   stored. If :python:`None`, the data are stored under Calzone's user data
   (i.e. :bash:`$HOME/.local/share/calzone/data`).

   .. note::

      In order to use (already available) `Geant4`_ data but located outside of
      Calzone's user space, the :bash:`$GEANT4_DATA_DIR` must be set
      accordingly.

----

.. autoclass:: calzone.Geant4Exception
   :members:

   This class represents an exception issued by the Geant4 kernel. Note however
   that Geant4 does not raise C++ exceptions, but instead relies on
   `G4VExceptionHandler`_. Therefore, Geant4 exceptions might be reported long
   *after* the scope that actually issued the error.

.. autoclass:: calzone.Geometry

   This class wraps an immutable `G4VPhysicalVolume`_ instance, acting as root
   (world) volume for the Geant4 Monte Carlo simulation.

   .. method:: __new__(definition)

      Create a new geometry instance from a *definition*. The :doc:`geometry
      definition <geometry>` can be provided directly as a Python :python:`dict`
      object, or loaded from a *definition* file (in `JSON`_, `TOML`_, or
      `YAML`_ format). For instance, the following creates a Monte Carlo
      geometry from a :python:`dict`-definition encoded in a `TOML`_ file.

      >>> geometry = calzone.Geometry("geometry.toml")

      .. seealso::

         The :py:class:`GeometryBuilder` class (described hereafter) allows for
         customisation of the geometry before actually building it.

   .. method:: __getitem__(self, pathname)

      Return an interface to a Monte Carlo :py:class:`Volume` given its absolute
      :ref:`pathname <pathname>` inside the geometry. For instance,

      >>> volume = geometry["Environment.Detector"]

   .. automethod:: check

      An integer *resolution* can be provided, specifying the number of Monte
      Carlo trials when looking for overlaps. The default resolution is of
      :python:`1000` trials per couple of volumes.

      On failure, i.e. as soon as an overlap is found, a
      :py:class:`Geant4Exception` is raised. Thus, only the first found overlap
      is reported, in case that the geometry comprises multiple overlaps.

   .. automethod:: display

      .. note::

         This method requires the `calzone-display`_ extension module to be
         installed.

      Launch an interactive display of the geometry. If tracking *data* is
      provided (as returned by the :py:meth:`Simulation.run` method), this
      information will be superimposed on the geometry display.

   .. automethod:: export()

      Export the Geant4 geometry as a `goupil.ExternalGeometry
      <ExternalGeometry_>`_.

   .. automethod:: find

      The *stem* argument might specify a volume :py:attr:`name
      <calzone.Volume.name>` or the tail of an incomplete :py:attr:`pathname
      <calzone.Volume.path>`.

   .. attribute:: root

      The geometry root :py:class:`volume <calzone.Volume>`.

----

.. autoclass:: calzone.GeometryBuilder

   This class manages a :doc:`geometry definition <geometry>`. It provides high
   level operators for customising the Monte Carlo geometry before actually
   building it.

   .. method:: __new__(definition)

      Create a new geometry *builder* from an initial *definition*, provided
      directly as a Python :python:`dict` object, or loaded from a *definition*
      file (in JSON, or TOML format). For instance,

      >>> builder = calzone.GeometryBuilder("geometry.toml")

   .. automethod:: build

      Upon successful completion, a :py:class:`Geometry` instance is returned,
      for instance as

      >>> geometry = builder.build()

      Note that the returned geometry is immutable. That is, subsequent
      *builder* operations do not modify the returned geometry.

   .. automethod:: delete

      The volume to remove is identified by its absolute :ref:`pathname
      <pathname>`. For instance, the following deletes the :python:`"Detector"`
      volume nested inside the root :python:`"Environment"` one.

      >>> builder.delete("Environment.Detector")

   .. automethod:: modify

      The volume to modify is identified by its absolute :ref:`pathname
      <pathname>`. The other arguments specify replacement values, if not
      :python:`None`. See :numref:`tab-volume-items` for the meaning of
      arguments. For instance, the following changes the *shape* of the root
      :python:`"Environment"` volume.

      >>> builder.modify("Environment", shape={"box": 1.0})

   .. automethod:: move

      *Source* and *destination* volumes are identified by their absolute
      :ref:`pathnames <pathname>`. For instance, the following renames the
      :python:`"Detector"` volume,

      >>> builder.move("Environment.Detector", "Environment.Scintillator")

      .. note:: the root volume cannot be moved, nor replaced, with this method.

   .. automethod:: place

      The volume *definition* can be provided directly as a Python dict object,
      or loaded from a definition file (in JSON, or TOML format). The *mother*
      argument specifies the location of the volume within the geometry
      hierarchy. Note that if a volume with the same name already exists at the
      given location, then it is replaced with the new *definition*. See the
      :doc:`geometry description <geometry>` section for the meaning of the
      other arguments.

   .. autoattribute:: algorithm

      Must be one of :python:`"bvh"` (default) or :python:`"geant4"`. Note that
      Geant4 native algorithm is inefficient for large meshes. Therefore, it
      should be avoided when a tessellated topography is used.

----

.. autoclass:: calzone.Map

   This class manages a regular grid of topography elevation values, e.g. from a
   Digital Elevation Model (DEM). Data are exposed as a mutable
   :external:py:class:`numpy.ndarray`, and can be exported as a 2D image (in PNG
   format), or as a 3D model (in STL format).

   .. method:: __new__(data)

      Create a new map instance from a grid of elevation values.

      The *data* can be provided as a `GeoTiff`_ object, or loaded from an image
      file (in TIFF, or PNG format). For instance, the following loads
      topography data from a GeoTIFF file,

      >>> topography = calzone.Map("topography.tif")

   .. automethod:: dump

      The export format is specified by the file extension. It must be one of
      :python:`".png"` or :python:`".stl"`. When exporting as STL, optional
      *kwargs* can be provided in order to customise the 3D shape (see
      :numref:`tab-topography-items`). For instance, the following exports the
      map as a PNG image (including topography metadata).

      >>> topography.dump("topography.png")

   .. automethod:: from_array

      The *z* argument must be a dim-2 :external:py:class:`numpy.ndarray` (of
      shape [:py:attr:`ny`, :py:attr:`nx`]) containing the topography elevation
      values (in C order, see the :py:attr:`z` attribute below). The *xlim* and
      *ylim* arguments are length-2 sequences specifying the map limits along
      the x and y-axis (i.e. :py:attr:`x0`, :py:attr:`x1`, :py:attr:`y0` and
      :py:attr:`y1` attributes below). For instance,

      >>> topography = calzone.Map.from_array(z, [x0, x1], [y0, y1])

      Optionally, the map :py:attr:`CRS <crs>` can also be indicated.

   .. note::

      The coordinates of map nodes :python:`(0,0)`, or :python:`(ny,nx)`, are
      :python:`(x0,y0)`, or :python:`(x1,y1)`, respectively. Since image formats
      are conventionally rendered with node :python:`(0,0)` located at upper
      left corner, it is frequent to have :python:`y0 > y1`.

   .. autoattribute:: crs

      This property is purely informative (and optional), since :py:class:`Map`
      objects do not allow for frame transforms.

   .. autoattribute:: nx
   .. autoattribute:: ny
   .. autoattribute:: x0
   .. autoattribute:: x1
   .. autoattribute:: y0
   .. autoattribute:: y1

   .. autoattribute:: z

      Elevation values are indexed in C-order. That is, :python:`z[i,j]`
      corresponds to the elevation at grid node :python:`j` along x-axis
      (columns), and :python:`i` along y-axis (rows).

----

.. autofunction:: calzone.particles

   This function returns a `structured numpy array <StructuredArray_>`_ with the
   given *shape*. Primary particles are initialised with default properties, if
   not overriden by specifying *kwargs*. For instance, the following creates an
   array of 100 primary particles (photons, by default) with a kinetic energy of
   0.5 MeV, starting from the origin (by default), and going downwards.

   >>> particles = calzone.particles(100, energy=0.5, direction=(0, 0, -1))

   The data structure (:external:py:class:`numpy.dtype`) of a primary particle
   is the following (the corresponding physical units are also indicated).

   .. list-table:: Primaries array structure.
      :width: 50%
      :widths: auto
      :header-rows: 1

      * - Field
        - Format
        - Units
      * - :python:`"pid"`
        - :python:`"i4"`
        - 
      * - :python:`"energy"`
        - :python:`"f8"`
        - MeV
      * - :python:`"position"`
        - :python:`"(f8, 3)"`
        - cm
      * - :python:`"direction"`
        - :python:`"(f8, 3)"`
        - 

   .. topic:: Particle ID

      The type of a Monte Carlo particle (:python:`"pid"`) is encoded according
      to the Particle Data Group (PDG) `numbering scheme <PdgScheme_>`_.

----

.. autoclass:: calzone.ParticlesGenerator

   This class provides a utility for the generation of Monte Carlo particles
   from configurable distributions. This tool is typically used to seed the
   Monte Carlo simulation with an initial set of particles.

   Once initialised, the generator can be further configured using the methods
   detailed below (i.e. following a `builder`_ pattern). The :py:func:`generate`
   method then triggers the actual sampling of Monte Carlo particles. As an
   example, the following generates N particles entering a specific *volume*,
   with a power-law energy distribution (between 10 |nbsp| keV and 10 |nbsp|
   MeV).

   >>> particles = simulation.particles()      \
   ...     .on(volume, direction="ingoing")    \
   ...     .powerlaw(1E-02, 1E+01)             \
   ...     .generate(N)

   .. method:: __new__(*, geometry=None, random=None, weight=True)

      Create a new particles generator.

      The *weight* argument specifies whether generation weights should be
      computed (i.e. the inverse of the generation likelihoods (:math:`\omega =
      1 / \text{pdf}(\text{S})`, for a Monte Carlo state :math:`\text{S}`) or
      not. Note that this can be overridden by individual distributions (using
      the :python:`weight` flag of other methods).

   .. automethod:: direction

      The direction is specified using Cartesian coordinates in the frame of the
      geometry root volume. For instance, the following sets the particles
      direction as upgoing along the (Oz) axis.

      >>> generator.direction([0, 0, 1])

   .. automethod:: energy

      For instance, the following sets the kinetic energy of Monte Carlo
      particles to 1 |nbsp| MeV.

      >>> generator.energy(1)

   .. automethod:: generate

      The *shape* argument defines the number of particles requested (as a
      :external:py:class:`ndarray <numpy.ndarray>` shape).

   .. automethod:: inside

      By default, the daughters volumes are excluded when generating the
      particles positions. Set the *include_daughters* flag to :python:`True` if
      this is not the desired behaviour.

   .. automethod:: on

      The optional *direction* argument is required to be one of
      :python:`"ingoing"` or :python:`"outgoing"`, if a value is provided. In
      this case, in addition to the position, the particle direction is also
      generated with respect to the surface normal, employing a cosine
      distribution over the half solid angle.

   .. automethod:: pid

      Monte Carlo particles are indentified by their Particle ID (PID), which
      follows the Particle Data Group (`PDG <PdgScheme_>`_) numbering scheme.

   .. automethod:: position

      The position is specified using Cartesian coordinates in the frame of the
      geometry root volume. For instance, the following sets the particles
      position 1 |nbsp| m above the origin.

      >>> generator.position([0, 0, 1E+02])

   .. automethod:: powerlaw

      The *energy_min* and *energy_max* arguments define the support of the
      power-law, as an interval.

      The default setting is a :math:`1 / E` power-lawx, corresponding to
      :python:`exponent=-1`. Note that setting the exponent value to zero
      results in a uniform distribution being used.

   .. automethod:: solid_angle

      The default settings is to consider the entire solid angle. The optional
      *theta* and *phi* arguments may be used to restrict the solid angle by
      specifying an interval of acceptable angular values, in deg.

   .. automethod:: spectrum

      The *data* argument specifies the spectral lines as a sequence of
      :python:`(energy, intensity)` tuples. For instance, the following defines
      a spectrum with two spectral lines (at 0.5 and 1 |nbsp| MeV) of equal
      intensities.

      >>> generator.spectrum([
      ...     (0.5, 1), # First line, at 0.5 MeV.
      ...     (1.0, 1), # Second line, at 1.0 MeV.
      ... ])

----

.. autoclass:: calzone.Physics

   .. method:: __new__(default_cut=None, em_model=None, had_model=None)

      Create a new set of Geant4 physics settings.

      See the :py:attr:`default_cut`, :py:attr:`em_model` and
      :py:attr:`had_model` attributes below for the meaning of the optional
      arguments. If an argument is left :python:`None`, then its default value
      is used. For instance, the following creates settings with standard
      electromagnetric physics (the default), and no hadronic physics.

      >>> physics = calzone.Physics()

   .. autoattribute:: default_cut

   .. autoattribute:: em_model

      Must be one of :python:`"dna"`, :python:`"livermore"`,
      :python:`"option1"`, :python:`"option2"`, :python:`"option3"`,
      :python:`"option4"`, :python:`"penelope"` or :python:`"standard"`
      (default). See the `Geant4 documentation <EMConstructors_>`_ for the
      meaning of these options.

      Setting :py:attr:`em_model` to :python:`None` disables the simulation of
      electromagnetic interactions.

      .. note::

         When :py:attr:`em_model` is not :python:`None`, then
         `G4EmExtraPhysics`_ is automatically enabled, in addition to the
         selected model of electromagnetic interactions.


   .. autoattribute:: had_model

      Must be one of :python:`"FTFP_BERT"`, :python:`"FTFP_BERT_HP"`,
      :python:`"QGSP_BERT"`, :python:`"QGSP_BERT_HP"`, :python:`"QGSP_BIC"` or
      :python:`"QGSP_BIC_HP"`. See the `Geant4 documentation
      <HadConstructors_>`_ for the meaning of these options.

      Setting :py:attr:`had_model` to :python:`None`, which is the default,
      disables the simulation of hadronic interactions.

----

.. autoclass:: calzone.Random

   This class exposes a stream of pseudo-random numbers as a cyclic sequence of
   :external:py:class:`float`. The stream is determined by the :py:attr:`seed`
   attribute, while the :py:attr:`index` attribute indicates its current state.

   .. note::

      A `Permuted Congruential Generator <WikipediaPCG_>`_ (PCG) is used (namely
      `Mcg128Xsl64`_), which has excellent performances for Monte Carlo
      applications.

   .. method:: __new__(seed=None)

      Create a new pseudo-random stream.

      If *seed* is :python:`None`, then a random value is picked using the
      system entropy. Otherwise, the specified :py:attr:`seed` value is used.
      For instance,

      >>> prng = calzone.Random(123456789)

   .. automethod:: uniform01

      If *shape* is :python:`None`, then a single number is returned. Otherwise,
      a :external:py:class:`numpy.ndarray` is returned, with the given *shape*.
      For instance, the following returns the next 100 pseudo-random
      numbers from the stream.

      >>> rns = prng.uniform01(100)

   .. autoattribute:: index

      This property can be modified, resulting in consuming or rewinding the
      pseudo-random stream. For instance, the following resets the stream.

      >>> prng.index = 0

   .. autoattribute:: seed

      The property fully determines (and identifies) the pseudo-random stream.
      Note that modifying the seed also resets the stream to index :python:`0`.

----

.. autoclass:: calzone.Simulation

   This class provides an interface for running a Geant4 simulation. The
   simulation is configured through a set of attributes described hereafter, or
   using the class constructor, below.

   .. method:: __new__(geometry=None, **kwargs)

      Create a new interface to a Geant4 simulation.

      The optional keyword arguments initialise the corresponding simulation
      attributes, described below. For instance, the following creates a new
      simulation interface with :py:attr:`tracking` enabled.

      >>> Simulation = calzone.simulation("geometry.toml", tracking=True)

   .. automethod:: particles

      The returned :py:class:`ParticlesGenerator` object is configured according
      to the simulation settings. Refer to the constructor of this object for
      further information.

   .. method:: run(particles, /)

      Run a Geant4 Monte Carlo simulation.

      The provided primary *particles* are transported through the Monte Carlo
      :py:attr:`geometry`, which must have been set first. The returned object
      depends on the simulation :py:attr:`sample_deposits`,
      :py:attr:`sample_particles` and :py:attr:`tracking` attributes. For
      example, if both deposits sampling and tracking are enabled, then a
      :external:py:class:`NamedTuple <typing.NamedTuple>` is returned,
      containing the sampled energy deposits, as well as the recorded tracks and
      vertices (as :external:py:class:`numpy.ndarray`, each).

   .. autoattribute:: geometry

      This property is a :py:class:`Geometry` instance. However, by default, no
      geometry is attached to the simulation.

   .. autoattribute:: physics

      This property is a :py:class:`Physics` instance. By default, only
      (:python:`"standard"`) electromagnetic interactions are enabled.

   .. autoattribute:: random

      This property is a :py:class:`Random` instance. By default, the
      pseudo-random stream is seeded using the system entropy.

   .. autoattribute:: sample_deposits

      Must be one of :python:`"brief"` (default) or :python:`"detailed"`. If set
      to :python:`None`, then energy deposits sampling is disabled for all
      volumes.

      In :python:`"brief"` mode, only the total energy deposits per active
      volume is recorded. On the contrary, in :python:`"detailed"` mode the full
      detail of energy deposition is reported.

   .. autoattribute:: sample_particles

      Must be a :python:`bool`, or :python:`None`. If :python:`False` (the
      default), then particles sampling is disabled at all volumes boundaries.

   .. autoattribute:: secondaries

      Must be a :python:`bool`, or :python:`None`. By default, secondary
      particles are enabled.

      .. tip::

         The deactivation of secondary particles can significantly speed up the
         Monte Carlo simulation, by orders of magnitude depending on the
         application. However, care must be exercised as it may be crucial to
         account for these secondary particles as part of the detector response.

   .. autoattribute:: tracking

      Must be a :python:`bool`, or :python:`None`. By default, Monte Carlo
      tracks recording is disabled.

----

.. autoclass:: calzone.Volume

   This class provides an interface for inspecting a `G4VPhysicalVolume`_ of an
   instanciated Monte Carlo geometry. Note that the geometry is static, i.e. it
   cannot be modified once it has been built, except for volume's
   :py:class:`role <Volume.role>`.

   :py:class:`Volume` objects are linked to a :py:class:`Geometry`, and cannot
   be instanciated directly. Instead, they are indexed from a geometry, e.g. as

   >>> volume = geometry["Environment.Detector"]

   .. automethod:: aabb

      The *frame* argument specifies the reference volume (by its absolute
      :ref:`pathname <pathname>`) of the axis-aligned bounding-box. If *frame*
      is :python:`None`, then the bounding box is computed in the root frame of
      the simulation. For instance, the following computes the volume AABB in
      its own frame (thus, the AABB of the underlying `G4VSolid`_, actually).

      >>> aabb = volume.aabb(volume.path)

   .. automethod:: display

      .. note::

         This method requires the `calzone-display`_ extension module to be
         installed.

      Launch an interactive display of the volume. If tracking *data* is
      provided (as returned by the :py:meth:`Simulation.run` method), this
      information will be superimposed on the geometry display.

   .. method:: dump(path=None)

      Dump the volume geometry to a `GDML`_ file.

      If *path* is :python:`None`, then the geometry is dumped to
      :python:`f"{self.name}.gdml"`. Note that contrary to Geant4, this method
      erases any existing `GDML`_ file with the same name.

   .. automethod:: origin

      As previously (see the :py:meth:`aabb` method), the *frame* argument
      specifies the reference volume. Note that depending on the underlying
      `G4VSolid`_, the origin might or might not be at the volume centre.

   .. automethod:: side

      The *element* argument can be a `structured numpy array
      <StructuredArray_>`_ containing the :python:`"position"` key (e.g. Monte
      Carlo :py:func:`particles <calzone.particles>`), or directy a
      :external:py:class:`numpy.ndarray` of cartesian coordinates.

      By default, points located inside daughter volumes are considered to be
      outside of the mother volume, when checking the side. Set
      *include_daughters* to :python:`True` if this is not the desired
      behaviour.

   .. autoattribute:: daughters

      Daughter's absolute :ref:`pathnames <pathname>` are returned, as a
      :external:py:class:`tuple` of :external:py:class:`str` objects. Note that
      only direct descendants are reported (e.g., not grand-daughters).

   .. autoattribute:: material

      This is the name of the underlying `G4Material`_, as registered to Geant4.

   .. autoattribute:: mother

   .. autoattribute:: name

      .. caution::

         The volume name **is not** guaranteed to be unique within a given
         geometry (see the :ref:`Geometry <geometry:Geometry structure>`
         section).

   .. autoattribute:: path

      .. tip::

         The volume pathname **is** guaranteed to be unique within a given
         geometry (see the :ref:`Geometry <geometry:Geometry structure>`
         section).

   .. autoattribute:: role

      See the the :ref:`geometry:roles` section for a list of potential volume
      roles.

      .. tip::

         Unlike other volume properties, roles can be modified after the Monte
         Carlo geometry has been built.

   .. autoattribute:: solid

      This is the Geant4 type (as a :external:py:class:`str`) of the underlying
      `G4VSolid`_.

   .. autoattribute:: surface_area


.. ============================================================================
.. 
.. URL links.
.. 
.. ============================================================================

.. _builder: https://en.wikipedia.org/wiki/Builder_pattern
.. _calzone-display: https://pypi.org/project/calzone-display
.. _EMConstructors: https://geant4-userdoc.web.cern.ch/UsersGuides/PhysicsListGuide/html/electromagnetic/index.html
.. _GDML: https://gdml.web.cern.ch/GDML/
.. _Geant4: https://geant4.web.cern.ch/docs/
.. _JSON: https://www.json.org/json-en.html
.. _HadConstructors: https://geant4-userdoc.web.cern.ch/UsersGuides/PhysicsListGuide/html/reference_PL/index.html
.. _ExternalGeometry: https://goupil.readthedocs.io/en/latest/py/external_geometry.html
.. _G4EmExtraPhysics: https://geant4.kek.jp/Reference/11.2.0/classG4EmExtraPhysics.html
.. _G4Material: https://geant4.kek.jp/Reference/11.2.0/classG4Material.html
.. _G4VExceptionHandler: https://geant4.kek.jp/Reference/11.2.0/classG4VExceptionHandler.html
.. _G4VPhysicalVolume: https://geant4.kek.jp/Reference/11.2.0/classG4VPhysicalVolume.html
.. _G4VSolid: https://geant4.kek.jp/Reference/11.2.0/classG4VSolid.html
.. _Geotiff: https://github.com/KipCrossing/geotiff
.. _Mcg128Xsl64: https://docs.rs/rand_pcg/latest/rand_pcg/struct.Mcg128Xsl64.html#
.. _PdgScheme: https://pdg.lbl.gov/2007/reviews/montecarlorpp.pdf
.. _StructuredArray: https://numpy.org/doc/stable/user/basics.rec.html
.. _TOML: https://toml.io/en/
.. _WikipediaPCG: https://en.wikipedia.org/wiki/Permuted_congruential_generator
.. _YAML: https://yaml.org/
