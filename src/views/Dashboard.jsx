import React from 'react';

export const Dashboard = ({ metrics }) => {
  const cpuLoad = isNaN(metrics.total_cpu_usage) ? 0 : metrics.total_cpu_usage;
  const cpuTemp = metrics.cpu_temperature !== undefined && metrics.cpu_temperature !== null && !isNaN(metrics.cpu_temperature) 
    ? metrics.cpu_temperature 
    : 0;
  const strokeDash = 251.2;

  const loadOffset = strokeDash - (cpuLoad / 100) * strokeDash;
  const tempOffset = strokeDash - ((cpuTemp || 40) / 100) * strokeDash;

  const formatUptime = (sec) => {
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return `${h}h ${m}m`;
  };

  return (
    <div className="dashboard-layout" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      
      {/* SECTION 1: SYSTEM PULSE (HERO) */}
      <div className="hero-grid">
        <div className="glass-card pulse-card" style={{ display: 'flex', flexDirection: 'column', gap: '24px', padding: '32px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
            <div>
              <div className="label" style={{ color: 'var(--brand-accent)', fontSize: '12px' }}>System Vitality Index</div>
              <h2 style={{ margin: '4px 0', fontSize: '28px', fontWeight: '800' }}>Live Performance Pulse</h2>
            </div>
            <div style={{ padding: '8px 16px', background: 'var(--brand-muted)', borderRadius: '20px', fontSize: '12px', fontWeight: '700', color: 'var(--brand-accent)' }}>
              {metrics.is_charging ? "🔌 High-Power Mode" : "🔋 Efficiency Mode"}
            </div>
          </div>

          <div style={{ display: 'flex', gap: '48px', alignItems: 'center', flexWrap: 'wrap' }}>
            {/* Main Load Gauge */}
            <div style={{ display: 'flex', gap: '20px', alignItems: 'center' }}>
              <div style={{ position: 'relative', width: '100px', height: '100px' }}>
                <svg width="100" height="100" viewBox="0 0 100 100" style={{ transform: 'rotate(-90deg)' }}>
                  <circle cx="50" cy="50" r="40" stroke="rgba(255,255,255,0.05)" strokeWidth="10" fill="transparent" />
                  <circle cx="50" cy="50" r="40" stroke="var(--brand-accent)" strokeWidth="10" fill="transparent"
                    strokeDasharray={strokeDash} strokeDashoffset={loadOffset} strokeLinecap="round" style={{ transition: 'stroke-dashoffset 0.8s ease' }} />
                </svg>
                <div style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)', fontSize: '18px', fontWeight: '900' }}>
                  {Math.round(cpuLoad)}%
                </div>
              </div>
              <div>
                <div className="label">Compute Load</div>
                <div className="value" style={{ fontSize: '32px' }}>{cpuLoad.toFixed(1)}%</div>
              </div>
            </div>

            {/* Power Drain */}
            <div style={{ display: 'flex', gap: '20px', alignItems: 'center' }}>
              <div style={{ width: '48px', height: '48px', background: 'var(--brand-muted)', borderRadius: '12px', display: 'flex', alignItems: 'center', justify: 'center' }}>
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--energy-amber)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
              </div>
              <div>
                <div className="label">Drainage</div>
                <div className="value" style={{ fontSize: '32px', color: 'var(--energy-amber)' }}>
                  {metrics.battery_discharge_rate ? `${Math.abs(metrics.battery_discharge_rate).toFixed(1)}W` : "0.0W"}
                </div>
              </div>
            </div>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(150px, 1fr))', gap: '20px', marginTop: '16px', borderTop: '1px solid var(--border)', paddingTop: '24px' }}>
             <div className="mini-stat">
                <span className="label">Memory Engine</span>
                <span className="val" style={{ color: 'var(--text-main)' }}>{(metrics.memory_used / 1024 / 1024 / 1024).toFixed(1)}GB</span>
             </div>
             <div className="mini-stat">
                <span className="label">Thermal Vitals</span>
                <span className="val" style={{ color: cpuTemp > 70 ? 'var(--thermal-hot)' : 'var(--text-main)' }}>{cpuTemp ? `${cpuTemp.toFixed(1)}°C` : 'N/A'}</span>
             </div>
             <div className="mini-stat">
                <span className="label">System Cycle</span>
                <span className="val" style={{ color: 'var(--text-main)' }}>{formatUptime(metrics.uptime)}</span>
             </div>
          </div>
        </div>

        <div className="group-card">
          <div className="label">System Strategy</div>
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '12px' }}>
            <div className="stat-card" style={{ padding: '16px', background: 'rgba(255,255,255,0.02)' }}>
              <div className="label" style={{ fontSize: '10px' }}>Active Power Tier</div>
              <div style={{ fontSize: '18px', fontWeight: '800', color: 'var(--brand-accent)', textTransform: 'uppercase', marginTop: '4px' }}>
                {metrics.daemon_tier ? metrics.daemon_tier.replace('Tier::', '') : "STANDARD"}
              </div>
            </div>
            <div className="stat-card" style={{ padding: '16px', background: 'rgba(255,255,255,0.02)' }}>
              <div className="label" style={{ fontSize: '10px' }}>Core Utilization</div>
              <div style={{ fontSize: '18px', fontWeight: '800', marginTop: '4px' }}>
                {metrics.daemon_unpark_count || 0} / {metrics.cores.length} <span style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>Online</span>
              </div>
            </div>
            <div className="stat-card" style={{ padding: '16px', background: 'rgba(255,255,255,0.02)' }}>
              <div className="label" style={{ fontSize: '10px' }}>Energy Engine</div>
              <div style={{ fontSize: '18px', fontWeight: '800', color: 'var(--success)', marginTop: '4px' }}>
                {metrics.config?.operation_mode === 'auto' || !metrics.config?.operation_mode ? 'AUTOPILOT' : 'STATIC'}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* SECTION 2: ENERGY INTELLIGENCE */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '24px' }}>
        <div className="group-card">
          <div className="label">Energy Intelligence (Battery Architecture)</div>
          <div className="metrics-row">
            <div className="stat-card">
              <div className="value" style={{ fontSize: '24px', color: 'var(--success)' }}>{metrics.battery_health ? `${metrics.battery_health.toFixed(1)}%` : "100%"}</div>
            </div>
            <div className="stat-card">
              <div className="label">Charge Loops</div>
              <div className="value" style={{ fontSize: '24px', color: 'var(--frequency-cyan)' }}>{metrics.battery_cycles || 0}</div>
            </div>
          </div>
          <div style={{ marginTop: '8px', padding: '16px', background: 'rgba(255,255,255,0.02)', borderRadius: '12px', border: '1px solid var(--border)' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
              <span className="label" style={{ fontSize: '10px' }}>Cell Chemistry</span>
              <span style={{ fontSize: '12px', fontWeight: '700' }}>{metrics.technology || "Lithium-Ion"}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span className="label" style={{ fontSize: '10px' }}>Model Identifier</span>
              <span style={{ fontSize: '12px', fontWeight: '700' }}>{metrics.model_name || "Primary"}</span>
            </div>
          </div>
        </div>

        <div className="group-card">
          <div className="label">Hardware Identity & Sub-States</div>
          <div className="stat-card">
            <div className="label">Fabricator</div>
            <div className="value" style={{ fontSize: '22px' }}>{metrics.manufacturer || "Generic"}</div>
            <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '4px' }}>Serial: {metrics.serial_number || "Internal Only"}</div>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
            <div className="stat-card" style={{ padding: '12px' }}>
               <div className="label" style={{ fontSize: '9px' }}>USB Autosuspend</div>
               <div style={{ fontSize: '14px', fontWeight: '800', color: metrics.config?.usb_autosuspend ? 'var(--success)' : 'var(--text-secondary)' }}>
                 {metrics.config?.usb_autosuspend ? "ENABLED" : "DISABLED"}
               </div>
            </div>
            <div className="stat-card" style={{ padding: '12px' }}>
               <div className="label" style={{ fontSize: '9px' }}>SATA ALPM</div>
               <div style={{ fontSize: '14px', fontWeight: '800', color: metrics.config?.sata_alpm ? 'var(--success)' : 'var(--text-secondary)' }}>
                 {metrics.config?.sata_alpm ? "ACTIVE" : "INACTIVE"}
               </div>
            </div>
          </div>
        </div>
      </div>

      {/* SECTION 3: CORE ARCHITECTURE */}
      <div className="glass-card">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
          <div className="label">Micro-Architecture Frequency Distribution ({metrics.cores.length} Cores)</div>
          <div style={{ fontSize: '11px', color: 'var(--text-secondary)', background: 'var(--brand-muted)', padding: '4px 8px', borderRadius: '4px' }}>
            Live Oscillator Tracking
          </div>
        </div>
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(140px, 1fr))',
          gap: '12px'
        }}>
          {metrics.cores.map((core) => {
            const isOffline = core.frequency <= 0;
            return (
              <div key={core.id} style={{
                background: isOffline ? 'rgba(255, 0, 0, 0.01)' : 'rgba(255, 255, 255, 0.02)',
                border: isOffline ? '1px dashed rgba(255,255,255,0.1)' : '1px solid var(--border)',
                borderRadius: '12px',
                padding: '16px',
                display: 'flex',
                flexDirection: 'column',
                gap: '8px',
                transition: 'all 0.3s ease'
              }}>
                <div style={{ fontSize: '10px', fontWeight: '700', color: 'var(--text-secondary)', textTransform: 'uppercase', display: 'flex', justifyContent: 'space-between' }}>
                  <span>Core {core.id}</span>
                  {isOffline && <span style={{ color: 'var(--energy-amber)', fontSize: '9px' }}>● PARKED</span>}
                </div>
                <div style={{ fontSize: '18px', fontWeight: '900', color: isOffline ? 'var(--text-secondary)' : 'var(--frequency-cyan)' }}>
                  {isOffline ? '0000' : core.frequency} 
                  <span style={{ fontSize: '10px', fontWeight: '400', color: 'var(--text-secondary)', marginLeft: '4px' }}>MHz</span>
                </div>
                <div style={{ width: '100%', height: '4px', background: 'rgba(255,255,255,0.05)', borderRadius: '2px' }}>
                  <div style={{
                    height: '100%',
                    width: `${isOffline ? 0 : Math.min(100, (core.frequency / 5000) * 100)}%`,
                    background: 'var(--brand-accent)',
                    borderRadius: '2px',
                    boxShadow: isOffline ? 'none' : '0 0 8px var(--brand-accent)',
                    transition: 'width 0.4s cubic-bezier(0.4, 0, 0.2, 1)'
                  }}></div>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
