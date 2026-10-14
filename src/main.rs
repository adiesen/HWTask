use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

/// Represents a single hardware data point (e.g., "CPU Core 0 Temperature")
#[derive(Debug, Clone)]
pub struct Sensor {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub unit: String,
}

/// The "Registry" is the heart of the app, mimicking HWiNFO's data tree
pub struct HardwareRegistry {
    pub sensors: Arc<Mutex<HashMap<String, Sensor>>>,
}

impl HardwareRegistry {
    pub fn new() -> Self {
        Self {
            sensors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Update or insert a sensor value
    pub fn update_sensor(&self, sensor: Sensor) {
        let mut sensors = self.sensors.lock().unwrap();
        sensors.insert(sensor.id.clone(), sensor);
    }
}

/// A Trait that every hardware component (CPU, GPU, RAM) must implement
pub trait HardwareCollector: Send + Sync {
    fn collect(&self, registry: &HardwareRegistry);
}

// --- EXAMPLE COLLECTOR: CPU ---
struct CpuCollector;
impl HardwareCollector for CpuCollector {
    fn collect(&self, registry: &HardwareRegistry) {
        // In a real app, this would read from /sys/class/hwmon/
        // or use a library like `sysinfo`
        let mock_temp = 55.0 + (rand::random::<f64>() * 5.0);

        registry.update_sensor(Sensor {
            id: "cpu_temp_0".to_string(),
            name: "CPU Package Temp".to_string(),
            value: mock_temp,
            unit: "°C".to_string(),
        });
    }
}

// --- EXAMPLE COLLECTOR: GPU ---
struct GpuCollector;
impl HardwareCollector for GpuCollector {
    fn collect(&self, registry: &HardwareRegistry) {
        let mock_usage = 40.0 + (rand::random::<f64>() * 10.0);

        registry.update_sensor(Sensor {
            id: "gpu_usage".to_string(),
            name: "GPU Utilization".to_string(),
            value: mock_usage,
            unit: "%".to_string(),
        });
    }
}

struct MemoryCollector;
impl HardwareCollector for MemoryCollector {
    fn collect(&self, registry: &HardwareRegistry) {
        registry.update_sensor(Sensor {
            id: "mem_usage".to_string(),
            name: "Memory Usage".to_string(),
            value: 12.5,
            unit: "%".to_string(),
        });
    }
}

fn main() -> eframe::Result<()> {
    let collectors: Vec<Box<dyn HardwareCollector>> = vec![
        Box::new(CpuCollector),
        Box::new(GpuCollector),
        Box::new(MemoryCollector),
    ];

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 760.0])
            .with_min_inner_size([860.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Pulse | Hardware Monitor",
        options,
        Box::new(move |creation_context| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(19, 25, 29);
            visuals.window_fill = egui::Color32::from_rgb(19, 25, 29);
            visuals.override_text_color = Some(egui::Color32::from_rgb(231, 238, 235));
            creation_context.egui_ctx.set_visuals(visuals);
            Ok(Box::new(HardwareApp::new(collectors)))
        }),
    )
}

struct HardwareApp {
    registry: HardwareRegistry,
    collectors: Vec<Box<dyn HardwareCollector>>,
    started: Instant,
    last_refresh: Instant,
}

impl HardwareApp {
    fn new(collectors: Vec<Box<dyn HardwareCollector>>) -> Self {
        let registry = HardwareRegistry::new();
        for collector in &collectors {
            collector.collect(&registry);
        }

        Self {
            registry,
            collectors,
            started: Instant::now(),
            last_refresh: Instant::now(),
        }
    }

    fn refresh(&mut self) {
        for collector in &self.collectors {
            collector.collect(&self.registry);
        }
        self.last_refresh = Instant::now();
    }

    fn sensors(&self) -> Vec<Sensor> {
        let mut sensors: Vec<Sensor> = self
            .registry
            .sensors
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect();
        sensors.sort_by(|left, right| left.id.cmp(&right.id));
        sensors
    }
}

impl eframe::App for HardwareApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_refresh.elapsed() >= Duration::from_secs(1) {
            self.refresh();
        }

        let sensors = self.sensors();
        let cpu_temp = sensor_value(&sensors, "cpu_temp_0");
        let gpu_usage = sensor_value(&sensors, "gpu_usage");
        let memory_usage = sensor_value(&sensors, "mem_usage");
        let uptime = self.started.elapsed().as_secs();
        let updated_seconds_ago = self.last_refresh.elapsed().as_secs();

        egui::CentralPanel::default().show(context, |ui| {
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("PULSE  /  HARDWARE MONITOR")
                            .size(13.0)
                            .strong()
                            .color(egui::Color32::from_rgb(91, 211, 193)),
                    );
                    ui.heading("System overview");
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Refresh now").clicked() {
                        self.refresh();
                    }
                    ui.label(
                        egui::RichText::new(format!("UPDATED {updated_seconds_ago}s AGO"))
                            .small()
                            .color(egui::Color32::from_rgb(151, 166, 164)),
                    );
                });
            });

            ui.add_space(16.0);
            ui.horizontal_wrapped(|ui| {
                ui.label("Live sensor snapshot");
                ui.separator();
                ui.label(format!("UPTIME  {:02}:{:02}", uptime / 60, uptime % 60));
                ui.separator();
                ui.label(
                    egui::RichText::new("SIMULATED DATA")
                        .strong()
                        .color(egui::Color32::from_rgb(241, 188, 97)),
                );
            });
            ui.add_space(18.0);

            ui.columns(3, |columns| {
                metric_card(
                    &mut columns[0],
                    "CPU PACKAGE TEMPERATURE",
                    cpu_temp,
                    "°C",
                    cpu_temp / 100.0,
                    egui::Color32::from_rgb(241, 188, 97),
                );
                metric_card(
                    &mut columns[1],
                    "GPU UTILIZATION",
                    gpu_usage,
                    "%",
                    gpu_usage / 100.0,
                    egui::Color32::from_rgb(89, 202, 211),
                );
                metric_card(
                    &mut columns[2],
                    "MEMORY USAGE",
                    memory_usage,
                    "%",
                    memory_usage / 100.0,
                    egui::Color32::from_rgb(127, 211, 151),
                );
            });

            ui.add_space(22.0);
            ui.heading("Sensor readings");
            ui.add_space(8.0);
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(25, 33, 37))
                .show(ui, |ui| {
                    egui::Grid::new("sensor_readings")
                        .num_columns(4)
                        .striped(true)
                        .spacing([24.0, 12.0])
                        .show(ui, |ui| {
                            for heading in ["SENSOR", "READING", "UNIT", "SOURCE"] {
                                ui.label(
                                    egui::RichText::new(heading)
                                        .small()
                                        .strong()
                                        .color(egui::Color32::from_rgb(151, 166, 164)),
                                );
                            }
                            ui.end_row();

                            for sensor in &sensors {
                                ui.label(&sensor.name);
                                ui.label(format!("{:.1}", sensor.value));
                                ui.label(&sensor.unit);
                                ui.label(
                                    egui::RichText::new("SIMULATED")
                                        .small()
                                        .color(egui::Color32::from_rgb(151, 166, 164)),
                                );
                                ui.end_row();
                            }
                        });
                });

            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(
                    "Hardware readings are placeholders and do not reflect this device.",
                )
                .small()
                .color(egui::Color32::from_rgb(151, 166, 164)),
            );
        });

        context.request_repaint_after(Duration::from_millis(250));
    }
}

fn metric_card(
    ui: &mut egui::Ui,
    title: &str,
    value: f64,
    unit: &str,
    fraction: f64,
    accent: egui::Color32,
) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(25, 33, 37))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_height(122.0);
            ui.label(
                egui::RichText::new(title)
                    .small()
                    .strong()
                    .color(egui::Color32::from_rgb(151, 166, 164)),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{value:.1}"))
                        .size(30.0)
                        .strong(),
                );
                ui.label(egui::RichText::new(unit).size(15.0).color(accent));
            });
            ui.add_space(8.0);
            ui.add(
                egui::ProgressBar::new(fraction.clamp(0.0, 1.0) as f32)
                    .desired_height(8.0)
                    .fill(accent),
            );
        });
}

fn sensor_value(sensors: &[Sensor], id: &str) -> f64 {
    sensors
        .iter()
        .find(|sensor| sensor.id == id)
        .map_or(0.0, |sensor| sensor.value)
}
