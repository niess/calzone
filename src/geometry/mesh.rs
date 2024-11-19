use bvh::aabb::{Aabb, Bounded};
use bvh::bounding_hierarchy::BHShape;
use bvh::bvh::{Bvh, BvhNode};
use nalgebra::{Point3, Vector3};
use bvh::ray::Ray;
use super::ffi;


pub struct SortedFacets {
    envelope: Aabb<f64, 3>,
    facets: Vec<TriangularFacet>,
    tree: Bvh::<f64, 3>,
    area: f64,
}

#[derive(Debug)]
struct TriangularFacet {
    v0: Point3<f64>,
    v1: Point3<f64>,
    v2: Point3<f64>,
    normal: Vector3<f64>,
    area: f64,
    index: usize,
}

#[derive(Clone, Copy)]
enum Side {
    Back,
    Both,
    Front,
}

impl TriangularFacet {
    fn closest(&self, p: &Point3<f64>) -> Point3<f64> {
        // Located closest point inside triangle.
        // Ref: https://stackoverflow.com/a/74395029

        let a = &self.v0;
        let b = &self.v1;
        let c = &self.v2;

        let ab = b - a;
        let ac = c - a;
        let ap = p - a;

        let d1 = ab.dot(&ap);
        let d2 = ac.dot(&ap);
        if (d1 <= 0.0) && (d2 <= 0.0) {
            return *a; //#1
        }

        let bp = p - b;
        let d3 = ab.dot(&bp);
        let d4 = ac.dot(&bp);
        if (d3 >= 0.0) && (d4 <= d3) {
            return *b; //#2
        }

        let cp = p - c;
        let d5 = ab.dot(&cp);
        let d6 = ac.dot(&cp);
        if (d6 >= 0.0) && (d5 <= d6) {
            return *c; //#3
        }

        let vc = d1 * d4 - d3 * d2;
        if (vc <= 0.0) && (d1 >= 0.0) && (d3 <= 0.0) {
            let v = d1 / (d1 - d3);
            return a + v * ab; //#4
        }

        let vb = d5 * d2 - d1 * d6;
        if (vb <= 0.0) && (d2 >= 0.0) && (d6 <= 0.0) {
            let v = d2 / (d2 - d6);
            return a + v * ac; //#5
        }

        let va = d3 * d6 - d5 * d4;
        if (va <= 0.0) && ((d4 - d3) >= 0.0) && ((d5 - d6) >= 0.0) {
            let v = (d4 - d3) / ((d4 - d3) + (d5 - d6));
            return b + v * (c - b); //#6
        }

        let denom = 1.0 / (va + vb + vc);
        let v = vb * denom;
        let w = vc * denom;
        return a + v * ab + w * ac; //#0
    }

    fn distance(&self, p: &Point3<f64>) -> f64 {
        let c = self.closest(p);
        (p - c).norm()
    }

    fn intersect(&self, ray: &Ray<f64, 3>, side: Side) -> Option<f64> {
        // From bvh crate, modified in order to manage back-culling.
        let a_to_b = self.v1 - self.v0;
        let a_to_c = self.v2 - self.v0;
        let u_vec = ray.direction.cross(&a_to_c);
        let det = a_to_b.dot(&u_vec);

        let hit = match side {
            Side::Back => det < -f64::EPSILON,
            Side::Both => det.abs() >= f64::EPSILON,
            Side::Front => det > f64::EPSILON,
        };
        if !hit {
            return None
        }

        let inv_det = 1.0 / det;
        let a_to_origin = ray.origin - self.v0;
        let u = a_to_origin.dot(&u_vec) * inv_det;

        if !(0.0..=1.0).contains(&u) {
            return None
        }

        let v_vec = a_to_origin.cross(&a_to_b);
        let v = ray.direction.dot(&v_vec) * inv_det;
        if v < 0.0 || u + v > 1.0 {
            return None
        }

        let distance = a_to_c.dot(&v_vec) * inv_det;
        if distance > f64::EPSILON {
            Some(distance)
        } else {
            None
        }
    }

    fn new(v0: Point3<f64>, v1: Point3<f64>, v2: Point3<f64>) -> Self {
        let u = v1 - v0;
        let v = v2 - v0;
        let mut normal = u.cross(&v);
        let norm = normal.norm();
        normal /= norm;
        let area = 0.5 * norm;
        let index = 0;
        Self { v0, v1, v2, normal, area, index }
    }
}

impl Bounded<f64, 3> for TriangularFacet {
    fn aabb(&self) -> Aabb<f64, 3> {
        let mut aabb = Aabb::empty();
        aabb.grow_mut(&self.v0);
        aabb.grow_mut(&self.v1);
        aabb.grow_mut(&self.v2);
        aabb
    }
}

impl BHShape<f64, 3> for TriangularFacet {
    fn set_bh_node_index(&mut self, index: usize) {
        self.index = index;
    }

    fn bh_node_index(&self) -> usize {
        self.index
    }
}

impl SortedFacets {
    pub fn area(&self) -> f64 {
        self.area
    }

    pub fn distance_to_in(
        &self,
        point: &ffi::G4ThreeVector,
        direction: &ffi::G4ThreeVector
    ) -> f64 {
        let point = Point3::new(point.x(), point.y(), point.z());
        let direction = Vector3::new(direction.x(), direction.y(), direction.z());
        let ray = Ray::new(point, direction);
        let (_, distance) = self.intersect(&ray, Side::Front);
        distance
    }

    pub fn distance_to_out(
        &self,
        point: &ffi::G4ThreeVector,
        direction: &ffi::G4ThreeVector,
        index: &mut i64,
    ) -> f64 {
        *index = -1;
        let point = Point3::new(point.x(), point.y(), point.z());
        let direction = Vector3::new(direction.x(), direction.y(), direction.z());
        let ray = Ray::new(point, direction);
        let (hits, distance) = self.intersect(&ray, Side::Back);
        if hits == 0 {
            0.0 // This should not happen, up to numeric uncertainties. In this case, Geant4
                // seems to return 0.
        } else {
            distance
        }
    }

    pub fn envelope(&self) -> [[f64; 3]; 2] {
        [
            [self.envelope.min[0], self.envelope.min[1], self.envelope.min[2]],
            [self.envelope.max[0], self.envelope.max[1], self.envelope.max[2]],
        ]
    }

    pub fn inside(&self, point: &ffi::G4ThreeVector, delta: f64) -> ffi::EInside {
        // First, let us check if the point lies on the surface (according to Geant4).
        struct Match {
            distance: f64,
        }

        impl Match {
            fn inspect(
                &mut self,
                sorted_facets: &SortedFacets,
                node_index: usize,
                point: &Point3<f64>,
                delta: f64,
            ) {
                match &sorted_facets.tree.nodes[node_index] {
                    BvhNode::Leaf{shape_index, ..} => {
                        let facet = &sorted_facets.facets[*shape_index];
                        let d = facet.distance(point);
                        if d < self.distance {
                            self.distance = d;
                        }
                    },
                    BvhNode::Node{child_l_index, child_l_aabb,
                                  child_r_index, child_r_aabb, ..} => {
                        if child_l_aabb.approx_contains_eps(&point, delta) {
                            self.inspect(sorted_facets, *child_l_index, point, delta)
                        }
                        if child_r_aabb.approx_contains_eps(&point, delta) {
                            self.inspect(sorted_facets, *child_r_index, point, delta)
                        }
                    },
                }
            }
        }

        let point = Point3::new(point.x(), point.y(), point.z());
        let mut closest = Match { distance: f64::INFINITY };
        closest.inspect(self, 0, &point, delta);
        if closest.distance <= delta {
            return ffi::EInside::kSurface;
        }

        // Otherwise, let us check if the point actually lies outside of the bounding box.
        if !self.envelope.approx_contains_eps(&point, delta) {
            return ffi::EInside::kOutside;
        }

        // Finally, let us count the number of intersections with the bounding surface. An odd
        // value implies an inner point.
        let direction = Vector3::new(0.0, 0.0, 1.0);
        let ray = Ray::new(point, direction);
        let (hits, _) = self.intersect(&ray, Side::Both);
        if (hits % 2) == 1 { ffi::EInside::kInside } else { ffi::EInside::kOutside }
    }

    fn intersect(&self, ray: &Ray<f64, 3>, side: Side ) -> (usize, f64) {
        struct Match<'a> {
            tree: &'a Bvh<f64, 3>,
            facets: &'a [TriangularFacet],
            side: Side,
            intersections: usize,
            distance: f64,
        }

        impl<'a> Match<'a> {
            fn inspect(&mut self, ray: &Ray<f64, 3>, index: usize) {
                match self.tree.nodes[index] {
                    BvhNode::Node {
                        ref child_l_aabb,
                        child_l_index,
                        ref child_r_aabb,
                        child_r_index,
                        ..
                    } => {
                        if ray_intersects_aabb(ray, child_l_aabb) {
                            self.inspect(ray, child_l_index);
                        }
                        if ray_intersects_aabb(ray, child_r_aabb) {
                            self.inspect(ray, child_r_index);
                        }
                    }
                    BvhNode::Leaf { shape_index, .. } => {
                        let facet = &self.facets[shape_index];
                        if let Some(distance) = facet.intersect(ray, self.side) {
                            self.intersections += 1;
                            if distance < self.distance {
                                self.distance = distance;
                            }
                        }
                    }
                }
            }
        }

        let mut matches = Match {
            side,
            tree: &self.tree,
            facets: self.facets.as_slice(),
            intersections: 0,
            distance: f64::INFINITY,
        };
        matches.inspect(ray, 0);
        (matches.intersections, matches.distance)
    }

    pub fn normal(&self, index: usize) -> [f64; 3] {
        self.facets[index].normal.into()
    }

    pub fn surface_normal(&self, point: &ffi::G4ThreeVector, delta: f64) -> [f64; 3] {
        struct Match {
            normal: [f64; 3],
            distance: f64,
        }

        impl Match {
            fn inspect(
                &mut self,
                sorted_facets: &SortedFacets,
                node_index: usize,
                point: &Point3<f64>,
                delta: f64,
            ) {
                match &sorted_facets.tree.nodes[node_index] {
                    BvhNode::Leaf{shape_index, ..} => {
                        let facet = &sorted_facets.facets[*shape_index];
                        let d = facet.distance(point);
                        if d < self.distance {
                            self.normal = facet.normal.into();
                            self.distance = d;
                        }
                    },
                    BvhNode::Node{child_l_index, child_l_aabb,
                                  child_r_index, child_r_aabb, ..} => {
                        if child_l_aabb.approx_contains_eps(&point, delta) {
                            self.inspect(sorted_facets, *child_l_index, point, delta)
                        }
                        if child_r_aabb.approx_contains_eps(&point, delta) {
                            self.inspect(sorted_facets, *child_r_index, point, delta)
                        }
                    },
                }
            }
        }

        let point = Point3::new(point.x(), point.y(), point.z());
        let mut closest = Match { normal: [0.0; 3], distance: f64::INFINITY };
        closest.inspect(self, 0, &point, delta);
        closest.normal
    }

    pub fn surface_point(&self, index: f64, u: f64, v: f64) -> [f64; 3] {
        let target = index * self.area;
        let mut area = 0.0;
        let mut index = self.facets.len() - 1;
        for (i, facet) in self.facets.iter().enumerate() {
            area += facet.area;
            if target <= area {
                index = i;
                break;
            }
        }
        let facet = &self.facets[index];
        let (u, v) = if u + v <= 1.0 { (u, v) } else { (1.0 - u, 1.0 - v) };
        let dr: Vector3<f64> = (u * (facet.v1 - facet.v0) + v * (facet.v2 - facet.v0)).into();
        let r = facet.v0 + dr;
        r.into()
    }
}

fn ray_intersects_aabb(ray: &Ray<f64, 3>, aabb: &Aabb<f64, 3>) -> bool {
    let lbr = (aabb[0].coords - ray.origin.coords).component_mul(&ray.inv_direction);
    let rtr = (aabb[1].coords - ray.origin.coords).component_mul(&ray.inv_direction);

    let (inf, sup) = lbr.inf_sup(&rtr);

    let tmin = inf.max();
    let tmin = if tmin > 0.0 { tmin } else { 0.0 };
    let tmax = sup.min();

    tmax >= tmin
}

pub fn sort_facets(shape: &ffi::MeshShape) -> Box<SortedFacets> {
    let data = &shape.facets;
    let mut envelope = Aabb::empty();
    let mut facets = Vec::<TriangularFacet>::with_capacity(data.len() / 9);
    let mut area = 0.0;
    for facet in data.chunks(9) {
        let [x0, y0, z0, x1, y1, z1, x2, y2, z2] = facet else { unreachable!() };
        const CM: f64 = 10.0;
        let v0 = Point3::<f64>::new(*x0 as f64 * CM, *y0 as f64 * CM, *z0 as f64 * CM);
        let v1 = Point3::<f64>::new(*x1 as f64 * CM, *y1 as f64 * CM, *z1 as f64 * CM);
        let v2 = Point3::<f64>::new(*x2 as f64 * CM, *y2 as f64 * CM, *z2 as f64 * CM);
        let facet = TriangularFacet::new(v0, v1, v2);
        area += facet.area;
        facets.push(facet);
        envelope.grow_mut(&v0);
        envelope.grow_mut(&v1);
        envelope.grow_mut(&v2);
    }
    let tree = Bvh::build(&mut facets);
    Box::new(SortedFacets { envelope, facets, tree, area })
}

impl From<&SortedFacets> for Vec<f32> {
    fn from(value: &SortedFacets) -> Self {
        let mut data = Vec::<f32>::with_capacity(9 * value.facets.len());
        for facet in value.facets.iter() {
            data.push(facet.v0[0] as f32);
            data.push(facet.v0[1] as f32);
            data.push(facet.v0[2] as f32);
            data.push(facet.v1[0] as f32);
            data.push(facet.v1[1] as f32);
            data.push(facet.v1[2] as f32);
            data.push(facet.v2[0] as f32);
            data.push(facet.v2[1] as f32);
            data.push(facet.v2[2] as f32);
        }
        data
    }
}
