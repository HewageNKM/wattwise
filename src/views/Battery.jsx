import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export const Battery = ({ metrics, formatTime, notify }) => {
  const [threshold, setThreshold] = useState(metrics.config.battery_threshold || 80);

  const updateThreshold = (val) => {
    setThreshold(val);
    invoke("set_battery_threshold", { start: 0, stop: parseInt(val) })
      .then(() => notify(\`Charge limit set to \${val}%\`))
      .catch(err => console.error(err));
  };

  const health = metrics.battery_health || 100;

  return (
    <div className="page-layout">
      <div className="main-pane">
        <div className="glass-card" style={{ marginBottom: '24px' }}>
          <div className="label">Primary Energy Reservoir</div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: '8px' }}>
            <span className="value" style={{ fontSize: '56px' }}>{metrics.battery_level}%</span>
            <span style={{ color: 'var(--text-secondary)', fontSize: '14px', fontWeight: '600' }}>{metrics.is_charging ? "Charging" : "Discharging"}</span>
          </div>
          
          <div className="metrics-row" style={{ marginTop: '32px' }}>
            <div className="stat-card">
              <div className="label">State of Health</div>
              <div className="value" style={{ fontSize: '24px', color: health > 80 ? 'var(--success)' : 'var(--energy-amber)' }}>
                {health.toFixed(1)}%
              </div>
            </div>
            <div className="stat-card">
              <div className="label">Cycle Integrity</div>
              <div className="value" style={{ fontSize: '24px', color: 'var(--frequency-cyan)' }}>{metrics.battery_cycles || "0"}</div>
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
                <span style={{ color: 'var(--text-secondary)' }}>MAX LIFESPAN (60%)</span>
                <span style={{ fontSize: '18px', fontWeight: '900', color: 'var(--brand-accent)' }}>{threshold}%</span>
                <span style={{ color: 'var(--text-secondary)' }}>MAX CAPACITY (100%)</span>
              </div>
           </div>
        </div>

        <div className="glass-card">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
             <div className="label">High-Precision Cell Telemetry</div>
             <span className="status-pill" style={{ background: 'var(--brand-muted)', color: 'var(--brand-accent)' }}>Live Bus Data</span>
          </div>
          
          <table className="vitals-table">
            <thead>
              <tr>
                <th>Telemetered Metric</th>
                <th>Standard Value</th>
                <th>Diagnostic Context</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td style={{ fontWeight: '600' }}>Line Potential</td>
                <td style={{ fontWeight: '800', color: 'var(--brand-accent)' }}>{metrics.battery_voltage?.toFixed(2) || "0.00"} V</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Current real-time voltage on the power bus.</td>
              </tr>
              <tr>
                <td style={{ fontWeight: '600' }}>Amperage Flow</td>
                <td style={{ fontWeight: '800', color: 'var(--energy-amber)' }}>{metrics.battery_current?.toFixed(3) || "0.000"} A</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Rate of electrons entering/exiting the reservoir.</td>
              </tr>
              <tr>
                <td style={{ fontWeight: '600' }}>Design Spec</td>
                <td>{(metrics.battery_capacity_design || 0).toFixed(2)} Wh/Ah</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Factory original chemical capacity.</td>
              </tr>
              <tr>
                <td style={{ fontWeight: '600' }}>Degradation Limit</td>
                <td>{(metrics.battery_capacity_full || 0).toFixed(2)} Wh/Ah</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Current maximum theoretical energy depth.</td>
              </tr>
              <tr>
                <td style={{ fontWeight: '600' }}>Hardware Architecture</td>
                <td>{metrics.manufacturer || "Generic"}</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Primary vendor identity.</td>
              </tr>
              <tr>
                <td style={{ fontWeight: '600' }}>Unit Serial</td>
                <td style={{ fontFamily: 'monospace', fontSize: '11px' }}>{metrics.serial_number || "N/A"}</td>
                <td style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>Unique Silicon/Cell signature code.</td>
              </tr>
            </tbody>
          </table>
          <div style={{ marginTop: '20px', padding: '12px', background: 'rgba(255, 255, 255, 0.02)', borderRadius: '8px', fontSize: '11px', color: 'var(--text-secondary)', textAlign: 'center' }}>
            Calculated Lifespan Efficiency: <strong style={{color: 'var(--brand-accent)'}}>{((metrics.battery_capacity_full / metrics.battery_capacity_design) * 100).toFixed(1)}%</strong> of original factory specification.
          </div>
        </div>
      </div>

      <div className="side-pane glass-card" style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
        <div>
          <div className="label">Energy Optimization</div>
          <p style={{ fontSize: '12px', color: 'var(--text-secondary)', lineHeight: '1.6', marginTop: '12px' }}>
            WattWise dynamically monitors discharge vectors to optimize the governor bias.
          </p>
        </div>
        <div style={{ padding: '16px', background: 'var(--brand-muted)', borderRadius: '12px', border: '1px solid var(--brand-accent)' }}>
          <div style={{ fontSize: '12px', fontWeight: '800', color: 'var(--brand-accent)' }}>💡 Tip</div>
          <p style={{ fontSize: '11px', margin: '4px 0 0', color: 'var(--text-main)' }}>
            Keeping your battery between 20-80% can double the useful lifespan of your cells.
          </p>
        </div>
      </div>
    </div>
  );
};
