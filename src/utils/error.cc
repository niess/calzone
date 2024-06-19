#include "calzone.h"
// Geant4 interface.
#include "G4StateManager.hh"
#include "G4VExceptionHandler.hh"


// ============================================================================
//
// Calzone interface.
//
// ============================================================================

static Error LAST_ERROR = { ErrorType::None, "" };
static std::shared_ptr<Error> POINTER {
    std::shared_ptr<Error>{}, &LAST_ERROR
};

bool any_error() {
    if (ctrlc_catched()) {
        LAST_ERROR.tp = ErrorType::KeyboardInterrupt;
        return true;
    } else {
        return LAST_ERROR.tp != ErrorType::None;
    }
}

void clear_error() {
    LAST_ERROR.tp = ErrorType::None;
}

void set_error(ErrorType tp, const char * message) {
    LAST_ERROR.tp = tp;
    LAST_ERROR.message = rust::String(message);
}

std::shared_ptr<Error> get_error() {
    return POINTER;
}


// ============================================================================
//
// Geant4 interface.
//
// ============================================================================

struct ExceptionImpl : public G4VExceptionHandler {

    // Geant4 interface.
    G4bool Notify(
        const char *, const char *, G4ExceptionSeverity, const char *
    );

    // User interface.
    static ExceptionImpl * Get();
};

G4bool ExceptionImpl::Notify(
    const char * origin,
    const char * code,
    G4ExceptionSeverity severity,
    const char * description) {
    switch (severity) {
        case G4ExceptionSeverity::FatalException:
        case G4ExceptionSeverity::FatalErrorInArgument:
            std::printf("%s(%s): %s\n", origin, code, description);
            return true;
        default:
            set_error(ErrorType::Geant4Exception, description);
            return false;
    }
}

ExceptionImpl * ExceptionImpl::Get() {
    static ExceptionImpl * instance = new ExceptionImpl();
    return instance;
}

void initialise_errors() {
    auto handler = ExceptionImpl::Get();
    G4StateManager::GetStateManager()->SetExceptionHandler(handler);
}
