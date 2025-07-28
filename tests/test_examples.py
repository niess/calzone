import os
from pathlib import Path
import pytest
import shutil
import subprocess
import sys
import tempfile


PREFIX = Path(__file__).parent.parent


def run(path, prepend=None):
    """Run example script."""

    path = PREFIX / f"examples/{path}"
    if prepend is not None:
        tmp = tempfile.TemporaryDirectory()
        shutil.copytree(path.parent, tmp.name, dirs_exist_ok=True)

        path = Path(tmp.name) / path.name
        with path.open("a") as f:
            f.write(f"{os.linesep}{prepend}")

    command = f"{sys.executable} {path}"
    r = subprocess.run(command, shell=True, capture_output=True)
    if r.returncode != 0:
        print(r.stdout.decode())
        raise RuntimeError(r.stderr.decode())

@pytest.mark.example
@pytest.mark.requires_data
def test_benchmark_gamma():
    """Test the benchmark gamma example."""

    run("gamma/benchmark/run.py")

@pytest.mark.example
@pytest.mark.requires_data
def test_underwater_gamma():
    """Test the underwater gamma example."""

    run("gamma/underwater/run.py")

@pytest.mark.example
@pytest.mark.requires_data
@pytest.mark.requires_goupil
def test_goupil():
    """Test the mixed goupil example."""

    run("gamma/goupil/run.py")

@pytest.mark.example
def test_topography():
    """Test the topography example."""

    run("geometry/topography/generate.py")

    result = PREFIX / "examples/geometry/topography/meshes/terrain.png"
    assert(result.is_file())

@pytest.mark.example
@pytest.mark.requires_data
@pytest.mark.requires_display
def test_trajectograph_muons():
    """Test the trajectograph muons example."""

    run("muon/trajectograph/run.py", prepend="""
import calzone_display
calzone_display.close()
""")

@pytest.mark.example
@pytest.mark.requires_data
def test_underwater_muons():
    """Test the underwater muons example."""

    run("muon/underwater/run.py")
