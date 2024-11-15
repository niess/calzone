#include "random.h"

double RandomImpl::flat() {
    return this->context->next_open01();
}

void RandomImpl::flatArray(const int n, double * v) {
    for (int i = 0; i < n; i++, v++) {
        *v = this->context->next_open01();
    }
}

std::string RandomImpl::name() const {
    return std::string(this->context->prng_name());
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

std::array<size_t, 2> RandomImpl::GetIndex() const {
    return this->context->index();
}

void RandomImpl::SetIndex(std::array<size_t, 2> index) {
    return this->context->set_index(index);
}

void RandomImpl::SetContext(RandomContext & context_) {
    if (this->context == nullptr) {
        // Enable.
        this->context = &context_;
        this->altEngine = G4Random::getTheEngine();
        G4Random::setTheEngine(this);
    } else {
        // This should be unreachable.
        std::fputs("error: random context overlap", stderr);
        std::exit(EXIT_FAILURE);
    }
}

void RandomImpl::ReleaseContext() {
    if (this->context == nullptr) {
        // This should be unreachable.
        std::fputs("error: missing random context", stderr);
        std::exit(EXIT_FAILURE);
    } else {
        // Disable.
        this->context = nullptr;
        G4Random::setTheEngine(this->altEngine);
        this->altEngine = nullptr;
    }
}

RandomImpl * RandomImpl::Get() {
    static RandomImpl * instance = new RandomImpl();
    return instance;
}

void set_random_context(RandomContext & context) {
    RandomImpl::Get()->SetContext(context);
}

void release_random_context() {
    RandomImpl::Get()->ReleaseContext();
}
