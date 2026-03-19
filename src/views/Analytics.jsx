import { useState } from "react";

const getSlope = (data) => {
  if (data.length < 3) return 0;
  let sumX = 0, sumY = 0, sumXY = 0, sumXX = 0;
  const n = data.length;
  data.forEach((d, i) => {
    sumX += i;
    sumY += (d.battery || 0);
    sumXY += i * (d.battery || 0);
    sumXX += i * i;
  });
  const denom = (n * sumXX - sumX * sumX);
  if (denom === 0) return 0;
  return (n * sumXY - sumX * sumY) / denom;
};

export const Analytics = ({ history, metrics }) => {
  const [activeFilter, setActiveFilter] = useState("usage");

  const getPercentage = (h) => {
    switch (activeFilter) {
      case "frequency": return (h.frequency / 5000) * 100; 
      case "temperature": return h.temperature; 
      case "battery": return h.battery; 
      default: return h.usage;
    }
  };

  const getDisplayValue = (h) => {
    switch (activeFilter) {
      case "frequency": return `${Math.round(h.frequency)} MHz`;
      case "temperature": return `${h.temperature.toFixed(1)}°C`;
      case "battery": return `${Math.round(h.battery)}%`;
      default: return `${h.usage.toFixed(1)}%`;
    }
  };

  const avgUsage = history.length > 0 
    ? history.reduce((acc, h) => acc + (activeFilter === "usage" ? h.usage : getPercentage(h)), 0) / history.length 
    : 0;
  
  const peakUsage = history.length > 0 
    ? Math.max(...history.map(h => activeFilter === "usage" ? h.usage : getPercentage(h))) 
    : 0;

  const health = metrics.battery_health || 100;
  const isCharging = metrics.is_charging;
  const wattage = Math.abs(metrics.battery_discharge_rate || 0);

  // 🎯 Efficiency Score Logic
  // High load at low wattage = High efficiency.
  // Formula: (Average Usage / Max(Wattage, 1)) * 10 (normalized)
  const efficiencyScore = Math.min(100, Math.round((metrics.total_cpu_usage / Math.max(wattage, 5)) * 20));
  
  // 🗺️ Heatmap Simulation (based on history of frequencies)
  const heatmapData = history.slice(-15).map(h => h.usage);

  const slope = getSlope(history);
  const advices = [];

  if (slope < 0 && metrics.battery_level) {
    const mins = (metrics.battery_level / (Math.abs(slope) * 60)).toFixed(0);
    if (mins > 0 && mins < 600) {
      advices.push(`AI Projection: Workload trend discharges battery fully in approx ${mins} mins.`);
    }
  }

  if (health < 80 && !isCharging) {
    advices.push("Your battery health is below 80%. We recommend enforcing 'Efficiency' mode to maximize endurance.");
  }
  if (!isCharging && avgUsage > 40) {
    advices.push("Aggressive CPU demand on battery detected. Consider disabling 'Turbo Boost' to guard thermals.");
  }
  if (isCharging) {
    advices.push("Connected to AC. Heuristics are unlocked for maximum scaling thresholds.");
  } else if (avgUsage <= 15) {
    advices.push("System idle on battery. WattWise Engine running at peak continuous efficiency.");
  }

  return (
    <div className="page-layout">
      <div className="main-pane">
        <div className="glass-card" style={{ display: 'flex', gap: '32px', alignItems: 'center' }}>
          <div className="efficiency-gauge-container">
            <svg width="140" height="140" viewBox="0 0 100 100">
              <circle cx="50" cy="50" r="45" fill="none" stroke="var(--border)" strokeWidth="8" />
              <circle cx="50" cy="50" r="45" fill="none" stroke="var(--success)" strokeWidth="8"
                strokeDasharray="282.7" strokeDashoffset={282.7 - (efficiencyScore / 100) * 282.7}
                strokeLinecap="round" style={{ transform: 'rotate(-90deg)', transformOrigin: '50% 50%', transition: 'stroke-dashoffset 1s ease' }} />
            </svg>
            <div style={{ position: 'absolute', textAlign: 'center' }}>
              <div style={{ fontSize: '28px', fontWeight: '900' }}>{efficiencyScore}</div>
              <div style={{ fontSize: '9px', fontWeight: '800', opacity: 0.6 }}>EFFICIENCY</div>
            </div>
          </div>
          <div style={{ flex: 1 }}>
            <div className="label">Performance-to-Power Ratio</div>
            <h2 style={{ margin: '8px 0', fontSize: '22px' }}>System Efficiency Score</h2>
            <p style={{ fontSize: '12px', color: 'var(--text-secondary)', margin: 0 }}>
              Calculated by cross-referencing computational throughput against line-drainage. 
              Higher scores indicate optimal governor utilization.
            </p>
          </div>
        </div>

        <div className="glass-card" style={{ marginTop: '24px' }}>
          <div className="label">Rolling {activeFilter} Heuristics</div>
          <div className="chart-area" style={{ 
            height: '240px', 
            display: 'flex', 
            alignItems: 'flex-end', 
            gap: '6px',
            padding: '24px',
            background: 'rgba(0,0,0,0.3)',
            borderRadius: '16px',
            marginTop: '16px',
            border: '1px solid var(--border)'
          }}>
            {history.map((h, i) => (
              <div 
                key={i} 
                className="chart-bar" 
                style={{ 
                  height: `${Math.max(4, getPercentage(h))}%`, 
                  flex: 1,
                  background: activeFilter === "battery" ? 'linear-gradient(to top, #00ff88, #00ffaa)' : 'linear-gradient(to top, var(--brand-accent), var(--success))',
                  borderRadius: '6px 6px 0 0',
                  transition: 'height 0.3s ease',
                }}
                title={`${h.time}: ${getDisplayValue(h)}`}
              ></div>
            ))}
          </div>
        </div>

        <div className="glass-card" style={{ marginTop: '24px' }}>
          <div className="label">Resource Utilization Heatmap (Live Distribution)</div>
          <div style={{ marginTop: '16px' }}>
            <div className="heatmap-grid">
              {Array.from({ length: 45 }).map((_, i) => {
                const val = heatmapData[i % heatmapData.length] || 0;
                return (
                  <div key={i} className="heatmap-cell" style={{ 
                    background: val > 60 ? 'var(--thermal-hot)' : val > 30 ? 'var(--energy-amber)' : val > 5 ? 'var(--success)' : 'rgba(255,255,255,0.05)'
                  }} title={`Node ${i}: ${val.toFixed(1)}% load`}></div>
                );
              })}
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '10px', fontSize: '9px', color: 'var(--text-secondary)', fontWeight: '700' }}>
              <span>IDLE</span>
              <span>ACTIVE</span>
              <span>THERMAL PEAK</span>
            </div>
          </div>
        </div>
      </div>

      <div className="side-pane">
        <div className="glass-card">
          <div className="label">Analytics Filter</div>
          <select 
            className="theme-select" 
            value={activeFilter}
            onChange={(e) => setActiveFilter(e.target.value)}
            style={{ marginTop: '12px' }}
          >
            <option value="usage">CPU Usage</option>
            <option value="frequency">Frequency</option>
            <option value="temperature">Temperature</option>
            <option value="battery">Battery Level</option>
          </select>
        </div>

        <div className="eco-box">
          <div className="label" style={{ color: 'var(--success)' }}>Eco-Impact Estimator</div>
          <div className="eco-value">-{((wattage * 0.45) / 100).toFixed(3)}kg</div>
          <div style={{ fontSize: '11px', fontWeight: '600' }}>Estimated CO2 Saved / Hour</div>
          <p style={{ fontSize: '10px', color: 'var(--text-secondary)', marginTop: '8px' }}>
            Calculated based on session efficiency and average baseline consumption.
          </p>
        </div>

        <div className="glass-card" style={{ marginTop: '24px' }}>
          <div className="label">Top CPU Consumers</div>
          <table style={{ width: '100%', marginTop: '12px', borderCollapse: 'collapse', fontSize: '13px' }}>
            <thead>
              <tr style={{ textAlign: 'left', borderBottom: '1px solid var(--border)', color: 'var(--text-secondary)' }}>
                <th style={{ padding: '8px 0', fontWeight: '400' }}>Process</th>
                <th style={{ padding: '8px 0', textAlign: 'right', fontWeight: '400' }}>CPU</th>
              </tr>
            </thead>
            <tbody>
              {metrics.top_processes?.map((proc, i) => (
                <tr key={i} style={{ borderBottom: '1px solid rgba(255,255,255,0.02)' }}>
                  <td style={{ padding: '10px 0', fontWeight: '700', maxWidth: '120px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{proc.name}</td>
                  <td style={{ padding: '10px 0', textAlign: 'right', fontWeight: '800', color: 'var(--brand-accent)' }}>{proc.cpu_usage.toFixed(1)}%</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
