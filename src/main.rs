#![allow(dead_code, unused_variables)]
use std::{error::Error};

use plotters::{coord::Shift, prelude::*};
use euclid::*;

const G: f64 = 6.6743e-11;
const YEAR: f64 = 60.*60.*24.*365.25;


struct WorldM;
struct WorldV;
struct WorldA;
struct WorldF;

#[derive(Debug, Clone, Copy)]
struct PlanateryObject {
    name: &'static str,
    mass: f64,
    position: Point2D<f64, WorldM>,
    velocity: Vector2D<f64, WorldV>,
}

impl PlanateryObject {
    fn new(name: &'static str, mass: f64, position: Point2D<f64, WorldM>, velocity: Vector2D<f64, WorldV>) -> PlanateryObject {
        PlanateryObject{name, mass, position, velocity}
    }

    fn copy_with_change(&self, position: Point2D<f64, WorldM>, velocity: Vector2D<f64, WorldV>) -> PlanateryObject {
        PlanateryObject::new(self.name, self.mass, position, velocity)
    }

    fn get_grav_force_vec(&self, other: &PlanateryObject) -> Vector2D<f64, WorldF> {
        let force_between = self.get_force(other);
        let vect = (other.position-self.position).with_length(force_between).cast_unit::<WorldF>();
        vect
    }

    fn get_force(&self, other: &PlanateryObject) -> f64 {
        let dist = self.position.distance_to(other.position).max(1e6);
        let force = G*((self.mass*other.mass)/(dist.powi(2)));
        force
    }

    fn get_accel(&self, force: Vector2D<f64, WorldF>) -> Vector2D<f64, WorldA> {
        let curr_mag = force.length();
        force.with_length(curr_mag/self.mass).cast_unit()
    }

    fn get_accel_at(&self, pos: Point2D<f64, WorldM>, other_planets: &Vec<PlanateryObject>,) -> Vector2D<f64, WorldA> 
    {
        let new_planet = self.copy_with_change(pos, self.velocity);
        let total_force = other_planets.iter()
            .map(|other| new_planet.get_grav_force_vec(&other))
            .fold(Vector2D::zero(), |accum, next| {
                accum + next 
            });
        self.get_accel(total_force)
    }

    fn update<A>(self, other_objs: A, time_step: f64) -> PlanateryObject 
        where A: Iterator<Item = PlanateryObject>
    {
        let other_planets = other_objs.collect();

        let p0 = self.position;
        let v0 = self.velocity;

        let kv1 = self.get_accel_at(p0, &other_planets);
        let kp1 = v0;

        let k2_pos: Point2D<f64, WorldM> = p0 + (kp1 * (time_step/2.)).cast_unit();
        let kv2 = self.get_accel_at(k2_pos, &other_planets);
        let kp2 = v0 + (kv1 * (time_step / 2.0)).cast_unit();

        let k3_pos: Point2D<f64, WorldM> = p0 + (kp2 * (time_step/2.)).cast_unit();
        let kv3 = self.get_accel_at(k3_pos, &other_planets);
        let kp3 = v0 + (kv2 * (time_step / 2.0)).cast_unit();

        let k4_pos: Point2D<f64, WorldM> = p0 + (kp3 * (time_step)).cast_unit();
        let kv4 = self.get_accel_at(k4_pos, &other_planets);
        let kp4 = v0 + (kv3 * (time_step)).cast_unit();

        let v_slope = (kv1 + kv2 * 2.0 + kv3 * 2.0 + kv4) / 6.0;
        let p_slope = (kp1 + kp2 * 2.0 + kp3 * 2.0 + kp4) / 6.0;

        let new_v = v0 + (v_slope * time_step).cast_unit();
        let new_p = p0 + (p_slope * time_step).cast_unit();

        self.copy_with_change(new_p, new_v)
    }
}

struct TimeInstance {
    objects: Vec<PlanateryObject>,
    time: f64
}

impl TimeInstance {
    fn new(planets: Vec<PlanateryObject>, time: f64) -> Self {
        Self { objects: planets, time }
    }
}

struct World {
    objects: Vec<PlanateryObject>,
    time_instances: Vec<TimeInstance>,
    time_step: f64,
    current_time: f64,
    max_time: f64,
}

impl World {
    fn new(objects: Vec<PlanateryObject>, max_time: f64, time_step: f64) -> Self {
        Self { objects, time_instances: vec![], time_step, current_time:0 as f64, max_time }
    }

    fn update(&mut self) {
        let mut new_planets = Vec::new(); 
        for i in 0..self.objects.len() {
            let new_planet = self.objects[i].update(
                self.objects.iter().enumerate()
                .filter(|(index, _)| *index!=i)
                .map(|(_, x)| x.clone()), self.time_step);

            new_planets.push(new_planet);
        }

        self.objects = new_planets.clone();
        self.current_time += self.time_step;
        self.time_instances.push(TimeInstance::new(new_planets, self.current_time));
    }

    const fn done(&self) -> bool {
        self.current_time >= self.max_time
    }

    fn plot<DB>(&self, root: &DrawingArea<DB, Shift>) -> Result<(), Box<dyn std::error::Error>>
    where DB: DrawingBackend,
          DB::ErrorType: 'static,
    {
        let mut max_dist: f64 = 1.0; 
        for inst in &self.time_instances {
            for obj in &inst.objects {
                max_dist = max_dist.max(obj.position.x.abs()).max(obj.position.y.abs());
            }
        }

        // 2. Add 10% padding
        let limit = max_dist * 1.1;

        let mut chart = ChartBuilder::on(root)
            .margin(20)
            .caption("Planetary Orbits", ("sans-serif", 30).into_font())
            .build_cartesian_2d(-limit..limit, -limit..limit)?;

        chart.configure_mesh().draw()?;

        let num_planets = self.objects.len();

        for i in 0..num_planets {
            let path: Vec<(f64, f64)> = self.time_instances.iter()
                .map(|inst| {
                    let p = inst.objects[i].position;
                    (p.x, p.y)
                })
                .collect();


            let color = Palette99::pick(i).mix(0.8);


            chart.draw_series(LineSeries::new(
                path,
                color.stroke_width(1),
            ))?.label(format!("{}", {self.objects[i].name}))
                .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color.stroke_width(3)));
        }

        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;

        Ok(())
        
    }
}


fn main() -> Result<(), Box<dyn Error>>{
    let backend = BitMapBackend::new("data/3_body_problem.png", (800, 800));
    let root = backend.into_drawing_area();
    root.fill(&WHITE)?;


    let m = 2.0e30;
    let star1 = PlanateryObject::new("star1", m, point2(1.5e11, 0.), vec2(0., 15000.));
    let star2 = PlanateryObject::new("star2", m, point2(-1.5e11, 0.), vec2(0., -15000.));
    let star3 = PlanateryObject::new("star3", m, point2(0., 2.0e11), vec2(0., 0.));

    let sun     = PlanateryObject::new("sun", 1.989e30, point2(0.0, 0.0), vec2(1.0, 1.0));
    let mercury = PlanateryObject::new("mercury", 3.301e23, point2(5.79e10, 0.0), vec2(0.0, 47360.0));
    let venus   = PlanateryObject::new("venus", 4.867e24, point2(1.08e11, 0.0), vec2(0.0, 35020.0));
    let earth   = PlanateryObject::new("earth", 5.972e24, point2(1.49e11, 0.0), vec2(0.0, 29780.0));
    let mars    = PlanateryObject::new("mars", 6.390e23, point2(2.28e11, 0.0), vec2(0.0, 24070.0));
    let jupiter = PlanateryObject::new("jupiter", 1.898e27, point2(7.78e11, 0.0), vec2(0.0, 13070.0));
    let saturn  = PlanateryObject::new("saturn", 5.683e26, point2(1.43e12, 0.0), vec2(0.0, 9690.0));
    let uranus  = PlanateryObject::new("uranus", 8.681e25, point2(2.87e12, 0.0), vec2(0.0, 6810.0));
    let neptune = PlanateryObject::new("neptune", 1.024e26, point2(4.50e12, 0.0), vec2(0.0, 5430.0));

    // let planets = vec![sun, mercury, venus, earth, mars];
    let planets = vec![star1, star2, star3];

    let steps = 300000;
    let run_time = YEAR*0.3;
    let timestep = run_time/steps as f64;

    let mut world = World::new(planets, run_time, timestep);

    while !world.done() {
        world.update();
    }

    world.plot(&root)?;
    root.present()?;
    


    Ok(())
}
