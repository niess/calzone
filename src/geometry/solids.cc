#include "solids.h"
// Geant4 interface.
#include "Randomize.hh"


// ============================================================================
//
// Box wrapper
//
// ============================================================================

G4VSolid * Box::Clone() const {
    return new Box(*this);
}

G4ThreeVector Box::GetPointOnSurface() const {
    auto && fDx = this->GetXHalfLength();
    auto && fDy = this->GetYHalfLength();
    auto && fDz = this->GetZHalfLength();

    // From G4Box::GetPointOnSurface but using G4UniformRand.
    G4double sxy = fDx * fDy, sxz = fDx * fDz, syz = fDy * fDz;
    G4double select = (sxy + sxz + syz) * G4UniformRand();
    G4double u = 2.0 * G4UniformRand() - 1.0;
    G4double v = 2.0 * G4UniformRand() - 1.0;

    if (select < sxy) {
        return {
            u * fDx,
            v * fDy,
            (select < 0.5 * sxy) ? -fDz : fDz
        };
    } else if (select < sxy + sxz) {
        return {
            u * fDx,
            (select < sxy + 0.5 * sxz) ? -fDy : fDy,
            v * fDz
        };
    } else {
        return {
            (select < sxy + sxz + 0.5 * syz) ? -fDx : fDx,
            u * fDy,
            v * fDz
        };
    }
}


// ============================================================================
//
// Displaced solid wrapper
//
// ============================================================================

G4VSolid * DisplacedSolid::Clone() const {
    return new DisplacedSolid(*this);
}


// ============================================================================
//
// Orb wrapper
//
// ============================================================================

G4VSolid * Orb::Clone() const {
    return new Orb(*this);
}


// ============================================================================
//
// Sphere wrapper
//
// ============================================================================

G4VSolid * Sphere::Clone() const {
    return new Sphere(*this);
}

G4ThreeVector Sphere::GetPointOnSurface() const {
    auto && fRmax = this->GetOuterRadius();
    auto && fRmin = this->GetInnerRadius();
    auto && fDPhi = this->GetDeltaPhiAngle();
    auto && fSPhi = this->GetStartPhiAngle();
    auto && fDTheta = this->GetDeltaThetaAngle();
    auto && fSTheta = this->GetStartThetaAngle();
    auto && cosSTheta = this->GetCosStartTheta();
    auto && cosETheta = this->GetCosEndTheta();
    auto && sinSTheta = this->GetSinStartTheta();
    auto && sinETheta = this->GetSinEndTheta();
    auto eTheta = fSTheta + fDTheta;
    bool fFullPhiSphere = fDPhi >= CLHEP::twopi;

    // From G4Sphere::GetPointOnSurface but using G4UniformRand.
    G4double RR = fRmax*fRmax;
    G4double rr = fRmin*fRmin;

    // Find surface areas
    //
    G4double aInner   = fDPhi*rr*(cosSTheta - cosETheta);
    G4double aOuter   = fDPhi*RR*(cosSTheta - cosETheta);
    G4double aPhi     = (!fFullPhiSphere) ? fDTheta*(RR - rr) : 0.;
    G4double aSTheta  = (fSTheta > 0) ? 0.5*fDPhi*(RR - rr)*sinSTheta : 0.;
    G4double aETheta  = (eTheta < CLHEP::pi) ? 0.5*fDPhi*(RR - rr)*sinETheta : 0.;
    G4double aTotal   = aInner + aOuter + aPhi + aSTheta + aETheta;

    // Select surface and generate a point
    //
    G4double select = aTotal*G4UniformRand();
    G4double u = G4UniformRand();
    G4double v = G4UniformRand();
    if (select < aInner + aOuter)            // lateral surface
    {
        G4double r   = (select < aInner) ? fRmin : fRmax;
        G4double z   = cosSTheta + (cosETheta - cosSTheta)*u;
        G4double rho = std::sqrt(1. - z*z);
        G4double phi = fDPhi*v + fSPhi;
        return { r*rho*std::cos(phi), r*rho*std::sin(phi), r*z };
    }
    else if (select < aInner + aOuter + aPhi) // cut in phi
    {
        G4double phi   = (select < aInner + aOuter + 0.5*aPhi) ? fSPhi : fSPhi + fDPhi;
        G4double r     = std::sqrt((RR - rr)*u + rr);
        G4double theta = fDTheta*v + fSTheta;
        G4double z     = std::cos(theta);
        G4double rho   = std::sin(theta);
        return { r*rho*std::cos(phi), r*rho*std::sin(phi), r*z };
    }
    else                                     // cut in theta
    {
        G4double theta = (select < aTotal - aETheta) ? fSTheta : fSTheta + fDTheta;
        G4double r     = std::sqrt((RR - rr)*u + rr);
        G4double phi   = fDPhi*v + fSPhi;
        G4double z     = std::cos(theta);
        G4double rho   = std::sin(theta);
        return { r*rho*std::cos(phi), r*rho*std::sin(phi), r*z };
    }
}


// ============================================================================
//
// Subtraction solid wrapper
//
// ============================================================================

G4VSolid * SubtractionSolid::Clone() const {
    return new SubtractionSolid(*this);
}


// ============================================================================
//
// Tubs wrapper
//
// ============================================================================

G4VSolid * Tubs::Clone() const {
    return new Tubs(*this);
}

G4ThreeVector Tubs::GetPointOnSurface() const {
    auto && fRMin = this->GetInnerRadius();
    auto && fRMax = this->GetOuterRadius();
    auto && fDz = this->GetZHalfLength();
    auto && fDPhi = this->GetDeltaPhiAngle();
    auto && fSPhi = this->GetStartPhiAngle();

    // From G4Tubs::GetPointOnSurface but using G4UniformRand.
    G4double Rmax = fRMax;
    G4double Rmin = fRMin;
    G4double hz = 2.*fDz;       // height
    G4double lext = fDPhi*Rmax; // length of external circular arc
    G4double lint = fDPhi*Rmin; // length of internal circular arc

    // Set array of surface areas
    //
    G4double RRmax = Rmax * Rmax;
    G4double RRmin = Rmin * Rmin;
    G4double sbase = 0.5*fDPhi*(RRmax - RRmin);
    G4double scut = (fDPhi == CLHEP::twopi) ? 0. : hz*(Rmax - Rmin);
    G4double ssurf[6] = { scut, scut, sbase, sbase, hz*lext, hz*lint };
    ssurf[1] += ssurf[0];
    ssurf[2] += ssurf[1];
    ssurf[3] += ssurf[2];
    ssurf[4] += ssurf[3];
    ssurf[5] += ssurf[4];

    // Select surface
    //
    G4double select = ssurf[5]*G4UniformRand();
    G4int k = 5;
    k -= (G4int)(select <= ssurf[4]);
    k -= (G4int)(select <= ssurf[3]);
    k -= (G4int)(select <= ssurf[2]);
    k -= (G4int)(select <= ssurf[1]);
    k -= (G4int)(select <= ssurf[0]);

    // Generate point on selected surface
    //
    switch(k)
    {
        case 0: // start phi cut
        {
            G4double r = Rmin + (Rmax - Rmin)*G4UniformRand();
            return { r*cosSPhi, r*sinSPhi, hz*G4UniformRand() - fDz };
        }
        case 1: // end phi cut
        {
            G4double r = Rmin + (Rmax - Rmin)*G4UniformRand();
            return { r*cosEPhi, r*sinEPhi, hz*G4UniformRand() - fDz };
        }
        case 2: // base at -dz
        {
            G4double r = std::sqrt(RRmin + (RRmax - RRmin)*G4UniformRand());
            G4double phi = fSPhi + fDPhi*G4UniformRand();
            return { r*std::cos(phi), r*std::sin(phi), -fDz };
        }
        case 3: // base at +dz
        {
            G4double r = std::sqrt(RRmin + (RRmax - RRmin)*G4UniformRand());
            G4double phi = fSPhi + fDPhi*G4UniformRand();
            return { r*std::cos(phi), r*std::sin(phi), fDz };
        }
        case 4: // external lateral surface
        {
            G4double phi = fSPhi + fDPhi*G4UniformRand();
            G4double z = hz*G4UniformRand() - fDz;
            G4double x = Rmax*std::cos(phi);
            G4double y = Rmax*std::sin(phi);
            return { x,y,z };
        }
        case 5: // internal lateral surface
        {
            G4double phi = fSPhi + fDPhi*G4UniformRand();
            G4double z = hz*G4UniformRand() - fDz;
            G4double x = Rmin*std::cos(phi);
            G4double y = Rmin*std::sin(phi);
            return { x,y,z };
        }
    }
    return {0., 0., 0.};
}
