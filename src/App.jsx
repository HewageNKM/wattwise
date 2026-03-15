import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [metrics, setMetrics] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    const interval = setInterval(() => {
      invoke("get_metrics")
        .then((res) => {
          setMetrics(res);
          setError(null);
        })
        .catch((err) => setError(err));
    }, 1000);

    return () => clearInterval(interval);
  }, []);

  const setGovernor = (gov) => {
    invoke("set_governor", { governor: gov })
      .catch((err) => setError(err));
  };

  if (!metrics) return <div className="app-container">Loading...</div>;

  return (
    <div className="app-container">
      <header>
        <h1>auto-cpufreq</h1>
        <div style={{ color: metrics.is_charging ? "#4ade80" : "#94a3b8" }}>
          {metrics.is_charging ? "Charging" : "Discharging"}
        </div>
      </header>

      {error && <div style={{ color: "#ef4444", marginBottom: 16 }}>{error}</div>}

      <div className="metrics-grid">
        <div className="card">
          <div className="card-title">CPU Usage</div>
          <div className="card-value">{metrics.total_cpu_usage.toFixed(1)}%</div>
        </div>
        <div className="card">
          <div className="card-title">Load Avg</div>
          <div className="card-value">{metrics.load_avg[0].toFixed(2)}</div>
        </div>
        <div className="card">
          <div className="card-title">Battery</div>
          <div className="card-value">
            {metrics.battery_level ? `${metrics.battery_level}%` : "N/A"}
          </div>
        </div>
      </div>

      <div className="card" style={{ marginBottom: 32 }}>
        <div className="card-title">Cores</div>
        <div className="core-list">
          {metrics.cores.map((core) => (
            <div key={core.id} className="core-item">
              <span>Core {core.id}</span>
              <span>{core.usage.toFixed(1)}% / {core.frequency}MHz</span>
            </div>
          ))}
        </div>
      </div>

      <div className="controls">
        <button onClick={() => setGovernor("performance")}>Performance</button>
        <button className="secondary" onClick={() => setGovernor("powersave")}>Powersave</button>
        <button className="secondary" onClick={() => invoke("set_turbo", { enabled: true })}>Enable Turbo</button>
      </div>
    </div>
  );
}

export default App;
