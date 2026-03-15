export const Dashboard = ({ metrics }) => {
  const cpuLoad = metrics.total_cpu_usage;
  const cpuTemp = metrics.cpu_temperature;
  const strokeDash = 251.2; // 2 * pi * r (40)
  
  const loadOffset = strokeDash - (cpuLoad / 100) * strokeDash;
  const tempOffset = strokeDash - ((cpuTemp || 40) / 100) * strokeDash;

  const formatUptime = (sec) => {
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return `${h}h ${m}m`;
  };

  return (
    <div className="dashboard-layout" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Top Diagnostics Roll-up */}
      <div className="metrics-grid" style={{ 
        display: 'grid', 
        gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', 
        gap: '20px' 
      }}>
        {/* CPU Load Gauge */}
        <div className="stat-card" style={{ display: 'flex', alignItems: 'center', gap: '16px', position: 'relative' }}>
          <div style={{ position: 'relative', width: '60px', height: '60px' }}>
            <svg width="60" height="60" viewBox="0 0 100 100" style={{ transform: 'rotate(-90deg)' }}>
              <circle cx="50" cy="50" r="40" stroke="var(--border)" strokeWidth="8" fill="transparent" />
              <circle cx="50" cy="50" r="40" stroke="var(--brand-accent)" strokeWidth="8" fill="transparent" 
                strokeDasharray={strokeDash} strokeDashoffset={loadOffset} strokeLinecap="round" style={{ transition: 'stroke-dashoffset 0.5s ease' }} />
            </svg>
            <div style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)', fontSize: '12px', fontWeight: '800' }}>
              {Math.round(cpuLoad)}%
            </div>
          </div>
          <div>
            <div className="label">Live CPU Load</div>
            <div className="value" style={{ fontSize: '20px' }}>{cpuLoad.toFixed(1)}%</div>
          </div>
        </div>

        {/* Thermal Temperature Gauge */}
        <div className="stat-card" style={{ display: 'flex', alignItems: 'center', gap: '16px', position: 'relative' }}>
          <div style={{ position: 'relative', width: '60px', height: '60px' }}>
            <svg width="60" height="60" viewBox="0 0 100 100" style={{ transform: 'rotate(-90deg)' }}>
              <circle cx="50" cy="50" r="40" stroke="var(--border)" strokeWidth="8" fill="transparent" />
              <circle cx="50" cy="50" r="40" stroke="#ff4757" strokeWidth="8" fill="transparent" 
                strokeDasharray={strokeDash} strokeDashoffset={tempOffset} strokeLinecap="round" style={{ transition: 'stroke-dashoffset 0.5s ease' }} />
            </svg>
            <div style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)', fontSize: '11px', fontWeight: '800' }}>
              {cpuTemp ? `${Math.round(cpuTemp)}°` : '--'}
            </div>
          </div>
          <div>
            <div className="label">Thermal Vitals</div>
            <div className="value" style={{ fontSize: '20px' }}>{cpuTemp ? `${cpuTemp.toFixed(1)}°C` : 'N/A'}</div>
          </div>
        </div>

        <div className="stat-card">
          <div className="label">Memory Engine</div>
          <div className="value" style={{ fontSize: '20px' }}>{(metrics.memory_used / 1024 / 1024 / 1024).toFixed(1)} GB</div>
          <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>of {(metrics.memory_total / 1024 / 1024 / 1024).toFixed(1)} GB</div>
        </div>

        <div className="stat-card">
          <div className="label">Continuous Uptime</div>
          <div className="value" style={{ fontSize: '20px' }}>{formatUptime(metrics.uptime)}</div>
          <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>Current System Cycle</div>
        </div>

        <div className="stat-card">
          <div className="label">Active Power Mode</div>
          <div className="value" style={{ fontSize: '20px', color: 'var(--brand-accent)', textTransform: 'capitalize' }}>
            {metrics.config?.manual_override ? metrics.config.manual_override : "Auto-Pilot (AI)"}
          </div>
          <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>Current system-wide steering state</div>
        </div>

        <div className="stat-card">
          <div className="label">System Power Drainage</div>
          <div className="value" style={{ fontSize: '20px', color: metrics.battery_discharge_rate ? '#fb1' : 'var(--success)' }}>
            {metrics.battery_discharge_rate ? `${Math.abs(metrics.battery_discharge_rate).toFixed(1)} W` : "Live AC Feed"}
          </div>
          <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>Total Energetic Consumption</div>
        </div>

        <div className="stat-card">
          <div className="label">Battery Health</div>
          <div className="value" style={{ fontSize: '20px', color: metrics.battery_health > 80 ? 'var(--success)' : '#fb1' }}>
            {metrics.battery_health ? `${metrics.battery_health.toFixed(1)}%` : "100.0%"}
          </div>
          <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>Lifespan Cycle Efficiency</div>
        </div>

        <div className="stat-card">
          <div className="label">Charge Cycles</div>
          <div className="value" style={{ fontSize: '20px', color: '#00f2fe' }}>
            {metrics.battery_cycles !== undefined && metrics.battery_cycles !== null ? metrics.battery_cycles : "0"}
          </div>
          <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>Total battery discharge loops</div>
        </div>

        <div className="stat-card">
          <div className="label">Peripheral Sub-States</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', marginTop: '8px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px' }}>
              <span style={{ color: 'var(--text-secondary)' }}>USB Suspend</span>
              <span style={{ color: metrics.config?.ac_profile?.usb_autosuspend ? 'var(--success)' : 'var(--text-secondary)', fontWeight: 'bold' }}>
                {metrics.config?.ac_profile?.usb_autosuspend ? 'AUTO' : 'OFF'}
              </span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', marginTop: '4px' }}>
              <span style={{ color: 'var(--text-secondary)' }}>SATA ALPM</span>
              <span style={{ color: metrics.config?.ac_profile?.sata_alpm ? 'var(--success)' : 'var(--text-secondary)', fontWeight: 'bold' }}>
                {metrics.config?.ac_profile?.sata_alpm ? 'MED' : 'MAX'}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Proactive Mode Dashboard Banner */}
      <div className="glass-card" style={{ 
        background: 'linear-gradient(135deg, rgba(0, 112, 243, 0.1), rgba(0, 255, 136, 0.05))',
        border: '1px solid var(--brand-accent)',
        borderRadius: '16px',
        padding: '20px',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center'
      }}>
        <div>
          <div style={{ fontWeight: '800', fontSize: '14px', color: 'var(--text-main)', display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ width: '8px', height: '8px', background: 'var(--success)', borderRadius: '50%', boxShadow: '0 0 8px var(--success)' }}></span>
            Dynamic Heuristic Engine: Operational
          </div>
          <p style={{ margin: '4px 0 0 16px', fontSize: '12px', color: 'var(--text-secondary)' }}>
            The governor is tracking real-time thread migration to bias scale responsiveness.
          </p>
        </div>
      </div>

      {/* Grid Expansion for Cores */}
      <div className="glass-card">
        <div className="label" style={{ marginBottom: '16px' }}>Core Micro-Architecture Frequency Distribution ({metrics.cores.length} Cores)</div>
        <div style={{ 
          display: 'grid', 
          gridTemplateColumns: 'repeat(auto-fill, minmax(130px, 1fr))', 
          gap: '12px' 
        }}>
          {metrics.cores.map((core) => (
            <div key={core.id} style={{ 
              background: 'rgba(255, 255, 255, 0.02)', 
              border: '1px solid var(--border)', 
              borderRadius: '12px', 
              padding: '12px',
              display: 'flex',
              flexDirection: 'column',
              gap: '4px'
            }}>
              <div style={{ fontSize: '10px', fontWeight: '700', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Core {core.id}</div>
              <div style={{ fontSize: '14px', fontWeight: '800', color: 'var(--brand-accent)' }}>{core.frequency} <span style={{ fontSize: '10px', fontWeight: '400', color: 'var(--text-secondary)' }}>MHz</span></div>
              <div style={{ width: '100%', height: '3px', background: 'var(--border)', borderRadius: '2px', marginTop: '4px' }}>
                <div style={{ 
                  height: '100%', 
                  width: `${Math.min(100, (core.frequency / 5000) * 100)}%`, 
                  background: 'var(--brand-accent)', 
                  borderRadius: '2px',
                  transition: 'width 0.3s ease'
                }}></div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
