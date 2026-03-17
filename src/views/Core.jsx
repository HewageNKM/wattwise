import { invoke } from "@tauri-apps/api/core";

export const Core = ({ metrics, notify }) => {
    // Fallback to "auto" if config isn't loaded yet
    const activeMode = metrics.config?.operation_mode || "auto";

    return (
        <div className="page-layout">
            <div className="main-pane">
                {/* 🧠 Core Heuristics Engine */}
                <div className="glass-card settings-group">
                    <h3>Global Power Heuristics</h3>
                    <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '16px' }}>
                        Configure the baseline intent of the predictive engine. Core allocation and Turbo Boost are managed dynamically.
                    </p>
                    <div className="action-row" style={{ gap: '8px', display: 'flex', flexWrap: 'wrap' }}>
                        <button
                            className={activeMode === "auto" ? "btn-primary" : "btn-secondary"}
                            onClick={() => invoke("set_operation_mode", { mode: "auto" }).then(() => notify("Switched to Auto-Pilot (AI)")).catch(console.error)}
                            style={{ flex: '1', fontSize: '12px', padding: '10px' }}
                        >🤖 Auto-Pilot</button>
                        <button
                            className={activeMode === "performance" ? "btn-primary" : "btn-secondary"}
                            onClick={() => invoke("set_operation_mode", { mode: "performance" }).then(() => notify("Always Performance Locked")).catch(console.error)}
                            style={{ flex: '1', fontSize: '12px', padding: '10px' }}
                        >⚡ Performance</button>
                        <button
                            className={activeMode === "efficiency" ? "btn-primary" : "btn-secondary"}
                            onClick={() => invoke("set_operation_mode", { mode: "efficiency" }).then(() => notify("Always Efficiency Locked")).catch(console.error)}
                            style={{ flex: '1', fontSize: '12px', padding: '10px' }}
                        >🔋 Efficiency</button>
                    </div>
                    <div style={{ marginTop: '12px', padding: '10px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px', fontSize: '11px' }}>
                        <strong>Engine Status:</strong>
                        {activeMode === "performance" && " All cores unparked. Turbo Boost forced ON. Maximum throughput."}
                        {activeMode === "efficiency" && " Base of 2 cores. Scales up on demand. Turbo Boost forced OFF."}
                        {activeMode === "auto" && " Predictive burst scaling active. Turbo seamlessly engages on sustained heavy loads."}
                    </div>
                </div>

                {/* 🔌 Global Peripheral Savers */}
                <div className="glass-card settings-group" style={{ marginTop: '20px' }}>
                    <h3>Advanced Peripheral Savers</h3>
                    <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '16px' }}>
                        Control absolute passive energy-drop loops for peripherals across all modes.
                    </p>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px' }}>
                            <div>
                                <div style={{ fontSize: '13px', fontWeight: '600' }}>USB Autosuspend</div>
                                <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '4px' }}>Power down idle USB ports to save wattage.</div>
                            </div>
                            <button
                                className={metrics.config?.usb_autosuspend ? "btn-primary" : "btn-secondary"}
                                onClick={() => invoke("set_usb_autosuspend", { enabled: !metrics.config?.usb_autosuspend })
                                    .then(() => notify(`USB Autosuspend ${!metrics.config?.usb_autosuspend ? 'Enabled' : 'Disabled'}`))
                                    .catch(console.error)}
                                style={{ padding: '6px 16px', fontSize: '12px' }}
                            >{metrics.config?.usb_autosuspend ? "Enabled" : "Enable"}</button>
                        </div>

                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px' }}>
                            <div>
                                <div style={{ fontSize: '13px', fontWeight: '600' }}>SATA ALPM</div>
                                <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '4px' }}>Aggressive power states for SCSI/SATA storage links.</div>
                            </div>
                            <button
                                className={metrics.config?.sata_alpm ? "btn-primary" : "btn-secondary"}
                                onClick={() => invoke("set_sata_alpm", { enabled: !metrics.config?.sata_alpm })
                                    .then(() => notify(`SATA ALPM ${!metrics.config?.sata_alpm ? 'Enabled' : 'Disabled'}`))
                                    .catch(console.error)}
                                style={{ padding: '6px 16px', fontSize: '12px' }}
                            >{metrics.config?.sata_alpm ? "Enabled" : "Enable"}</button>
                        </div>
                    </div>
                </div>
            </div>

            {/* Side pane remains mostly the same, just updated text */}
            <div className="side-pane glass-card">
                <div className="label">Configuration Integrity</div>
                <div style={{ marginTop: '16px', fontSize: '11px', color: 'var(--text-secondary)', lineHeight: '1.6' }}>
                    <p>All settings are persisted to <code>/etc/zenith-energy/config.json</code> and take immediate effect.</p>
                    <p style={{ marginTop: '12px' }}>Your new intent-based engine dynamically overrides manual frequency caps to ensure system stability while maximizing battery life.</p>
                </div>
            </div>
        </div>
    );
};