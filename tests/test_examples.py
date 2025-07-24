from pathlib import Path
import pytest
import subprocess
import sys


PREFIX = Path(__file__).parent.parent


def run(path):
    """Run example script."""

    path = PREFIX / f"examples/{path}"
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

    run("muon/trajectograph/run.py")

@pytest.mark.example
@pytest.mark.requires_data
def test_underwater_muons():
    """Test the underwater muons example."""

    run("muon/underwater/run.py")
