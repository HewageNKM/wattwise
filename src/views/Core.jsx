import { invoke } from "@tauri-apps/api/core";

export const Core = ({ metrics, notify }) => {


    return (
        <div className="page-layout">
            <div className="main-pane">
                <div className="glass-card settings-group">
                    <h3>Global Power Heuristics</h3>
                    <p style={{fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '16px'}}>
                        Configure how the system prioritizes energy consumption vs computational throughput.
                    </p>
                    <div className="action-row" style={{gap: '8px', display: 'flex', flexWrap: 'wrap'}}>
                        <button 
                            className={!metrics.config?.manual_override ? "btn-primary" : "btn-secondary"} 
                            onClick={() => invoke("set_operation_mode", { mode: "auto" }).then(() => notify("Switched to Auto-Pilot (AI)")).catch(console.error)}
                            style={{flex: '1', fontSize: '12px', padding: '10px'}}
                        >🤖 Auto-Pilot</button>
                        <button 
                            className={metrics.config?.manual_override === "performance" ? "btn-primary" : "btn-secondary"} 
                            onClick={() => invoke("set_operation_mode", { mode: "performance" }).then(() => notify("Always Performance Locked")).catch(console.error)}
                            style={{flex: '1', fontSize: '12px', padding: '10px'}}
                        >⚡ Performance</button>
                        <button 
                            className={metrics.config?.manual_override === "efficiency" ? "btn-primary" : "btn-secondary"} 
                            onClick={() => invoke("set_operation_mode", { mode: "efficiency" }).then(() => notify("Always Efficiency Locked")).catch(console.error)}
                            style={{flex: '1', fontSize: '12px', padding: '10px'}}
                        >🔋 Efficiency</button>
                    </div>
                    {metrics.config?.manual_override && <p style={{fontSize: '10px', color: 'var(--success)', marginTop: '8px'}}>Manual Override: {metrics.config?.manual_override.toUpperCase()}</p>}
                    <div style={{marginTop: '12px', padding: '10px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px', fontSize: '11px'}}>
                        <strong>Note:</strong> 
                        {metrics.config?.manual_override === "performance" && " Forces AC high-performance profiles strictly, ignoring cable sensors."}
                        {metrics.config?.manual_override === "efficiency" && " Forces Battery deep powersave profiles strictly on all dimensions."}
                        {(!metrics.config?.manual_override) && " Dynamic auto-switching presets accurately based on active power source."}
                    </div>
                </div>

                {/* 🔌 AC Profile Settings */}
                <div className="glass-card settings-group" style={{marginTop: '20px'}}>
                    <h3>🔌 AC Profile Settings</h3>
                    <p style={{fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '16px'}}>
                        Configure static behaviors when connected to a live charger securely node layout.
                    </p>
                    <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px'}}>
                        <div>
                            <div style={{fontSize: '13px', fontWeight: '600'}}>Intel/AMD Turbo Boost</div>
                            <div style={{fontSize: '11px', color: 'var(--text-secondary)', marginTop: '4px'}}>Unlock full clock Potential on AC.</div>
                        </div>
                        <button 
                            className={metrics.config?.ac_profile?.turbo === true ? "btn-primary" : "btn-secondary"} 
                            onClick={() => invoke("set_profile_turbo", { profile: "ac", enabled: !metrics.config?.ac_profile?.turbo })
                                .then(() => notify(`AC Turbo ${!metrics.config?.ac_profile?.turbo ? 'Enabled' : 'Disabled'}`))
                                .catch(console.error)}
                            style={{padding: '6px 16px', fontSize: '12px'}}
                        >{metrics.config?.ac_profile?.turbo === true ? "Enabled" : "Enable"}</button>
                    </div>
                </div>

                {/* 🔋 Battery Profile Settings */}
                <div className="glass-card settings-group" style={{marginTop: '20px'}}>
                    <h3>🔋 Battery Profile Settings</h3>
                    <p style={{fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '16px'}}>
                        Configure static behaviors when running on battery cells layout securely node.
                    </p>
                    <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px'}}>
                        <div>
                            <div style={{fontSize: '13px', fontWeight: '600'}}>Intel/AMD Turbo Boost</div>
                            <div style={{fontSize: '11px', color: 'var(--text-secondary)', marginTop: '4px'}}>Cap frequencies to preserve capacity.</div>
                        </div>
                        <button 
                            className={metrics.config?.bat_profile?.turbo === true ? "btn-primary" : "btn-secondary"} 
                            onClick={() => invoke("set_profile_turbo", { profile: "bat", enabled: !metrics.config?.bat_profile?.turbo })
                                .then(() => notify(`Battery Turbo ${!metrics.config?.bat_profile?.turbo ? 'Enabled' : 'Disabled'}`))
                                .catch(console.error)}
                            style={{padding: '6px 16px', fontSize: '12px'}}
                        >{metrics.config?.bat_profile?.turbo === true ? "Enabled" : "Enable"}</button>
                    </div>
                </div>

                <div className="glass-card settings-group" style={{marginTop: '20px'}}>
                    <h3>Advanced Peripheral Savers</h3>
                    <p style={{fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '16px'}}>
                        Control absolute passive energy-drop loops for peripherals.
                    </p>
                    <div style={{display: 'flex', flexDirection: 'column', gap: '12px'}}>
                        <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px'}}>
                            <div>
                                <div style={{fontSize: '13px', fontWeight: '600'}}>USB Autosuspend</div>
                                <div style={{fontSize: '11px', color: 'var(--text-secondary)', marginTop: '4px'}}>Power down idle USB ports to save wattage.</div>
                            </div>
                            <button 
                                className={metrics.config?.usb_autosuspend === true ? "btn-primary" : "btn-secondary"} 
                                onClick={() => invoke("set_usb_autosuspend", { enabled: !metrics.config?.usb_autosuspend })
                                    .then(() => notify(`USB Autosuspend ${!metrics.config?.usb_autosuspend ? 'Enabled' : 'Disabled'}`))
                                    .catch(console.error)}
                                style={{padding: '6px 16px', fontSize: '12px'}}
                            >{metrics.config?.usb_autosuspend === true ? "Enabled" : "Enable"}</button>
                        </div>

                        <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px'}}>
                            <div>
                                <div style={{fontSize: '13px', fontWeight: '600'}}>SATA ALPM</div>
                                <div style={{fontSize: '11px', color: 'var(--text-secondary)', marginTop: '4px'}}>Aggressive power states for SCSI/SATA storage links.</div>
                            </div>
                            <button 
                                className={metrics.config?.sata_alpm === true ? "btn-primary" : "btn-secondary"} 
                                onClick={() => invoke("set_sata_alpm", { enabled: !metrics.config?.sata_alpm })
                                    .then(() => notify(`SATA ALPM ${!metrics.config?.sata_alpm ? 'Enabled' : 'Disabled'}`))
                                    .catch(console.error)}
                                style={{padding: '6px 16px', fontSize: '12px'}}
                            >{metrics.config?.sata_alpm === true ? "Enabled" : "Enable"}</button>
                        </div>
                    </div>
                </div>
            </div>
            
            <div className="side-pane glass-card">
                <div className="label">Configuration Integrity</div>
                <div style={{marginTop: '16px', fontSize: '11px', color: 'var(--text-secondary)', lineHeight: '1.6'}}>
                    <p>All settings are persisted to <code>/etc/zenith-energy/config.json</code> and take immediate effect on the background optimization service.</p>
                    <p style={{marginTop: '12px'}}>Manual overrides will prevent the AI from automatically switching modes when you plug/unplug your device.</p>
                </div>
                <div className="settings-group" style={{ marginTop: '24px' }}>
                    <p style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Persistence Engine</p>
                    <div style={{fontSize: '10px', color: 'var(--success)'}}>● Service Active</div>
                    <div style={{fontSize: '10px', opacity: 0.6}}>● Authored with Rust</div>
                </div>
            </div>
        </div>
    );
};
