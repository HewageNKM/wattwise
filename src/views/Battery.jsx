import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export const Battery = ({ metrics, formatTime, notify }) => {
  const [threshold, setThreshold] = useState(metrics.config.battery_threshold || 80);

  const updateThreshold = (val) => {
    setThreshold(val);
    invoke("set_battery_threshold", { start: 0, stop: parseInt(val) })
      .then(() => notify(`Charge limit set to ${val}%`))
      .catch(err => console.error(err));
  };

  const health = metrics.battery_health || 100;
  const wattage = Math.abs(metrics.battery_discharge_rate || 0);
  const timeSec = metrics.battery_time_remaining || 0;

  // 🧪 Health Projection Logic
  const getLifecycleStatus = (h, c) => {
    if (h > 95 && c < 50) return { label: "PRISTINE", color: "var(--success)" };
    if (h > 85) return { label: "HEALTHY", color: "var(--success)" };
    if (h > 75) return { label: "STABLE", color: "var(--energy-amber)" };
    return { label: "WORN", color: "var(--thermal-hot)" };
  };
  const status = getLifecycleStatus(health, metrics.battery_cycles || 0);

  return (
    <div className="page-layout">
      <div className="main-pane">
        <div className="glass-card" style={{ display: 'flex', gap: '40px', alignItems: 'center', marginBottom: '24px' }}>
          <div style={{ position: 'relative', width: '150px', height: '150px', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <svg width="150" height="150" viewBox="0 0 100 100">
               <circle cx="50" cy="50" r="45" fill="none" stroke="var(--border)" strokeWidth="6" />
               <circle cx="50" cy="50" r="45" fill="none" stroke={status.color} strokeWidth="6"
                 strokeDasharray="282.7" strokeDashoffset={282.7 - (health / 100) * 282.7}
                 strokeLinecap="round" style={{ transform: 'rotate(-90deg)', transformOrigin: '50% 50%', transition: 'stroke-dashoffset 1.5s ease' }} />
            </svg>
            <div style={{ position: 'absolute', textAlign: 'center' }}>
              <div style={{ fontSize: '32px', fontWeight: '900' }}>{health.toFixed(1)}%</div>
              <div style={{ fontSize: '9px', fontWeight: '800', opacity: 0.6 }}>HEALTH</div>
            </div>
          </div>
          <div style={{ flex: 1 }}>
            <div className="lifecycle-badge" style={{ background: `${status.color}22`, color: status.color, marginBottom: '12px' }}>
              <span>🛡️</span> {status.label} SYSTEM
            </div>
            <h2 style={{ margin: '0 0 8px', fontSize: '26px', fontWeight: '800' }}>Chemical Integrity Analysis</h2>
            <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-secondary)' }}>
              A real-time snapshot of your cell's maximum structural capacity relative to its factory-new state. 
              Cycle count: <strong style={{color: 'var(--text-main)'}}>{metrics.battery_cycles || 0}</strong>
            </p>
          </div>
        </div>

        <div className="glass-card" style={{ marginBottom: '24px' }}>
          <div className="label">Endurance Projections (Mode Impact)</div>
          <div className="endurance-grid">
            <div className="endurance-box" style={{ borderColor: 'var(--thermal-hot)' }}>
              <div className="label">Performance</div>
              <div className="endurance-time">{formatTime(timeSec * 0.7)}</div>
              <div style={{ fontSize: '10px', fontWeight: '700' }}>-30% Endurance</div>
            </div>
            <div className="endurance-box" style={{ borderColor: 'var(--brand-accent)' }}>
              <div className="label">Balanced</div>
              <div className="endurance-time">{formatTime(timeSec)}</div>
              <div style={{ fontSize: '10px', fontWeight: '700' }}>Base Baseline</div>
            </div>
            <div className="endurance-box" style={{ borderColor: 'var(--success)' }}>
              <div className="label">Efficiency</div>
              <div className="endurance-time">{formatTime(timeSec * 1.45)}</div>
              <div style={{ fontSize: '10px', fontWeight: '700' }}>+45% Endurance</div>
            </div>
          </div>
        </div>

        <div className="glass-card" style={{ marginBottom: '24px' }}>
          <div className="label">Energy Persistence Control (Charge Limiter)</div>
          <div style={{ padding: '24px 0 12px' }}>
             <input 
               type="range" 
               min="60" 
               max="100" 
               step="5" 
               value={threshold} 
               onChange={(e) => updateThreshold(e.target.value)}
               style={{ width: '100%', accentColor: 'var(--brand-accent)' }}
             />
             <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '12px', fontSize: '12px', fontWeight: '600' }}>
               <span style={{ color: 'var(--text-secondary)' }}>LIFESPAN FOCUS (60%)</span>
               <span style={{ fontSize: '20px', fontWeight: '900', color: 'var(--brand-accent)' }}>{threshold}%</span>
               <span style={{ color: 'var(--text-secondary)' }}>CAPACITY FOCUS (100%)</span>
             </div>
          </div>
        </div>

        <div className="glass-card">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
             <div className="label">High-Precision Telemetry</div>
             <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                <div style={{ textAlign: 'right' }}>
                  <div style={{ fontSize: '20px', fontWeight: '900', color: 'var(--energy-amber)' }}>{wattage.toFixed(1)}W</div>
                  <div style={{ fontSize: '9px', fontWeight: '700', opacity: 0.6 }}>DRAIN RATE</div>
                </div>
                <span className="status-pill" style={{ background: 'var(--brand-muted)', color: 'var(--brand-accent)' }}>Live Bus</span>
             </div>
          </div>
          
          <table className="vitals-table">
            <thead>
              <tr>
                <th>Bus Metric</th>
                <th>Current Value</th>
                <th>Diagnostic Context</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td style={{ fontWeight: '600' }}>Potential</td>
                <td style={{ fontWeight: '800', color: 'var(--brand-accent)' }}>{metrics.battery_voltage?.toFixed(2) || "0.00"} V</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Real-time line voltage on the primary bus.</td>
              </tr>
              <tr>
                <td style={{ fontWeight: '600' }}>Amperage Flow</td>
                <td style={{ fontWeight: '800', color: 'var(--energy-amber)' }}>{metrics.battery_current?.toFixed(3) || "0.000"} A</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Electron rate through the charging circuit.</td>
              </tr>
              <tr>
                <td style={{ fontWeight: '600' }}>Design Spec</td>
                <td>{(metrics.battery_capacity_design || 0).toFixed(2)} Wh/Ah</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Original chemical design depth.</td>
              </tr>
              <tr>
                <td style={{ fontWeight: '600' }}>Full Capacity</td>
                <td>{(metrics.battery_capacity_full || 0).toFixed(2)} Wh/Ah</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Current maximum usable energy depth.</td>
              </tr>
              <tr>
                <td style={{ fontWeight: '600' }}>Unit Serial</td>
                <td style={{ fontFamily: 'monospace', fontSize: '11px' }}>{metrics.serial_number || "N/A"}</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Hardware identification signature.</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div className="side-pane glass-card" style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
        <div>
          <div className="label">Optimization Info</div>
          <p style={{ fontSize: '12px', color: 'var(--text-secondary)', lineHeight: '1.6', marginTop: '12px' }}>
            WattWise dynamically monitors discharge vectors to optimize the governor bias.
          </p>
        </div>
        <div style={{ padding: '16px', background: 'var(--brand-muted)', borderRadius: '12px', border: '1px solid var(--brand-accent)' }}>
          <div style={{ fontSize: '12px', fontWeight: '800', color: 'var(--brand-accent)' }}>🔋 Optimization Tip</div>
          <p style={{ fontSize: '11px', margin: '4px 0 0', color: 'var(--text-main)', lineHeight: '1.5' }}>
            Limiting your charge to 80% can significantly slow the crystalline degradation of your battery cells over time.
          </p>
        </div>
        <div className="glass-card" style={{ padding: '16px' }}>
          <div className="label">Technology</div>
          <div style={{ fontSize: '14px', fontWeight: '800', marginTop: '8px' }}>{metrics.technology || "Li-ion"}</div>
          <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Chemistry Variant</div>
        </div>
      </div>
    </div>
  );
};
