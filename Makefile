GEANT4_VERSION= 11.2.2
GEANT4_URL= https://gitlab.cern.ch/geant4/geant4/-/archive/v$(GEANT4_VERSION)/geant4-v$(GEANT4_VERSION).tar.gz


manylinux2014:
	sudo singularity build --sandbox $@ docker://quay.io/pypa/manylinux2014_x86_64

geant4-$(GEANT4_VERSION):
	wget -qO- $(GEANT4_URL) | tar xvz
	mv geant4-v$(GEANT4_VERSION) $@

# Requires: `sudo singularity shell -w manylinux2014`
.PHONY: xerces-c
xerces-c:
	yum install -y xerces-c-devel

build:
	mkdir -p build &&                                                      \
	cd build &&                                                            \
	cmake ../geant4-$(GEANT4_VERSION)                                      \
	    -DCMAKE_INSTALL_PREFIX=$$PWD/geant4                                \
	    -DCMAKE_CXX_FLAGS=-std=c++17                                       \
	    -DGEANT4_BUILD_MULTITHREADED=OFF                                   \
	    -DGEANT4_USE_GDML=ON                                               \
	    -DGEANT4_INSTALL_DATA=OFF                                          \
            -DGEANT4_INSTALL_DATADIR=~/.local/share/calzone/data/

.PHONY: tarball
tarball:
	tar -cvzf geant4-$(GEANT4_VERSION)-manylinux2014-x86_64.tgz            \
	    geant4/bin                                                         \
	    geant4/include                                                     \
	    geant4/lib64
