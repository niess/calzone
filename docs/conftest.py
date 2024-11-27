import calzone
import numpy
import pytest


DOCTEST_INITIALISED = False

def initialise_doctest():
    """Initialise the doctest environement."""

    with open("geometry.toml", "w") as f:
        f.write("""\
[Environment.Detector]
box = 1.0
position = [0, 0, +0.5]

[Environment.Terrain]
box = 1.0
position = [0, 0, -0.5]
""")

    with open("materials.toml", "w") as f:
        f.write("\n")

    DOCTEST_INITIALISED = True


@pytest.fixture(autouse=True)
def _docdir(request, doctest_namespace):

    doctest_plugin = request.config.pluginmanager.getplugin("doctest")
    if isinstance(request.node, doctest_plugin.DoctestItem):
        doctest_namespace["calzone"] = calzone
        doctest_namespace["numpy"] = numpy
        tmpdir = request.getfixturevalue("tmpdir")
        with tmpdir.as_cwd():
            if not DOCTEST_INITIALISED:
                initialise_doctest()
            yield
    else:
        yield
