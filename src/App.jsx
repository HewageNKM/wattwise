import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [metrics, setMetrics] = useState(null);
  const [history, setHistory] = useState([]);
  const [error, setError] = useState(null);
  const [activeTab, setActiveTab] = useState("dashboard");

  const formatTime = (hours) => {
    if (!hours) return "Calculating...";
    const h = Math.floor(hours);
    const m = Math.round((hours - h) * 60);
    return `${h}h ${m}m`;
  };

  useEffect(() => {
    const interval = setInterval(() => {
      invoke("get_metrics")
        .then((res) => {
          setMetrics(res);
          setHistory(prev => [...prev.slice(-20), {
            time: new Date().toLocaleTimeString(),
            usage: res.total_cpu_usage,
            freq: res.cores[0]?.frequency || 0
          }]);
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
        <div className="nav">
          <button className={activeTab === "dashboard" ? "active" : "flat"} onClick={() => setActiveTab("dashboard")}>Dashboard</button>
          <button className={activeTab === "stats" ? "active" : "flat"} onClick={() => setActiveTab("stats")}>Analytics</button>
        </div>
        <div style={{ color: metrics.is_charging ? "#4ade80" : "#94a3b8", fontSize: "14px" }}>
          {metrics.is_charging ? "Charging" : "Discharging"}
        </div>
      </header>

      {error && <div style={{ color: "#ef4444", marginBottom: 16 }}>{error}</div>}

      {activeTab === "dashboard" ? (
        <>
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
              <div className="sub-stats">
                {metrics.battery_time_remaining && (
                  <div className="sub-stat">
                    <span className="label">Remaining</span>
                    <span className="val">{formatTime(metrics.battery_time_remaining)}</span>
                  </div>
                )}
                {metrics.battery_health && (
                  <div className="sub-stat">
                    <span className="label">Health</span>
                    <span className="val">{Math.round(metrics.battery_health)}%</span>
                  </div>
                )}
                {metrics.battery_cycles !== null && (
                  <div className="sub-stat">
                    <span className="label">Cycles</span>
                    <span className="val">{metrics.battery_cycles}</span>
                  </div>
                )}
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
        </>
      ) : (
        <div className="stats-view">
          <div className="card">
            <div className="card-title">Performance History</div>
            <div className="chart-container">
              {history.map((h, i) => (
                <div key={i} className="chart-bar-wrap">
                  <div className="chart-bar" style={{ height: `${h.usage}%` }}></div>
                  <span className="chart-label">{h.usage.toFixed(0)}</span>
                </div>
              ))}
            </div>
            <div className="chart-legend">
              CPU Usage (%) over last 20 seconds
            </div>
          </div>
        </div>
      )}

      <div className="controls">
        <button onClick={() => setGovernor("performance")}>Performance</button>
        <button className="secondary" onClick={() => setGovernor("powersave")}>Powersave</button>
        <button className="secondary" onClick={() => invoke("set_turbo", { enabled: true })}>Enable Turbo</button>
      </div>
    </div>
  );
}

export default App;
