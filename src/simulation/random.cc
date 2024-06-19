#include "random.h"


double RandomImpl::flat() {
    return RUN_AGENT->next_open01();
}

void RandomImpl::flatArray(const int n, double * v) {
    for (int i = 0; i < n; i++, v++) {
        *v = RUN_AGENT->next_open01();
    }
}

std::string RandomImpl::name() const {
    return std::string(RUN_AGENT->prng_name());
}

void RandomImpl::setSeed(long, int) {}
void RandomImpl::setSeeds(const long *, int) {}
void RandomImpl::saveStatus(const char *) const {}
void RandomImpl::restoreStatus(const char *) {}
void RandomImpl::showStatus() const {}

std::ostream & RandomImpl::put (std::ostream & os) const {
    return os;
}

std::istream & RandomImpl::get (std::istream &) {
    std::cerr << "error: call to unimplemented method RandomImpl::get"
              << std::endl;
    exit(EXIT_FAILURE);
}

void RandomImpl::Switch() {
    if (this->altEngine == nullptr) {
        // Enable.
        this->altEngine = G4Random::getTheEngine();
        G4Random::setTheEngine(this);
    } else {
        // Disable.
        G4Random::setTheEngine(this->altEngine);
        this->altEngine = nullptr;
    }
}

RandomImpl * RandomImpl::Get() {
    static RandomImpl * instance = new RandomImpl();
    return instance;
}
