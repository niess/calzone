#pragma once
// Geant4 interface.
#include "Randomize.hh"
// User interface.
#include "calzone.h"

struct RandomImpl: public CLHEP::HepRandomEngine {
    RandomImpl(const RandomImpl &) = delete;

    // Geant4 interface.
    double flat();
    void flatArray(const int, double *);
    std::string name() const;

    void setSeed(long, int);
    void setSeeds(const long *, int);
    void saveStatus(const char *) const;
    void restoreStatus(const char *);
    void showStatus() const;

    std::ostream & put (std::ostream & os) const;
    std::istream & get (std::istream & is);

    // User interface.
    std::array<size_t, 2> GetIndex() const;
    void SetIndex(std::array<size_t, 2>);
    void SetContext(RandomContext &);
    void ReleaseContext();

    static RandomImpl * Get();

private:
    RandomImpl() = default;

    RandomContext * context = nullptr;
    CLHEP::HepRandomEngine * altEngine = nullptr;
};
