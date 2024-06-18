#include "random.h"


RandomImpl::RandomImpl(): CLHEP::HepRandomEngine() {}

double RandomImpl::flat() {
    return this->agent->next_open01();
}

void RandomImpl::flatArray(const int n, double * v) {
    for (int i = 0; i < n; i++, v++) {
        *v = this->agent->next_open01();
    }
}

std::string RandomImpl::name() const {
    return std::string(this->agent->prng_name());
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

void RandomImpl::Configure(RunAgent & runAgent) {
    this->agent = &runAgent;
}

RandomImpl * RandomImpl::Get() {
    static RandomImpl * instance = new RandomImpl();
    return instance;
}
