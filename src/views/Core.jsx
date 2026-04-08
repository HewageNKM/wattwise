import { invoke } from "@tauri-apps/api/core";

export const Core = ({ metrics, notify }) => {
    const activeMode = metrics.config?.operation_mode || "auto";

    return (
        <div className="page-layout">
            <div className="main-pane">
                <div className="glass-card" style={{ marginBottom: '24px' }}>
                    <div className="label">Global Power Heuristics</div>
                    <h2 style={{ margin: '8px 0 20px', fontSize: '24px', fontWeight: '800' }}>Engine Intent Control</h2>
                    <div className="action-row" style={{ display: 'flex', gap: '12px' }}>
                        <button
                            className={activeMode === "auto" ? "btn-primary" : "btn-secondary"}
                            onClick={() => invoke("set_operation_mode", { mode: "auto" }).then(() => notify("Auto-Pilot Engaged")).catch(console.error)}
                            style={{ flex: 1, padding: '16px' }}
                        >🤖 Auto-Pilot</button>
                        <button
                            className={activeMode === "performance" ? "btn-primary" : "btn-secondary"}
                            onClick={() => invoke("set_operation_mode", { mode: "performance" }).then(() => notify("Performance Locked")).catch(console.error)}
                            style={{ flex: 1, padding: '16px' }}
                        >⚡ Performance</button>
                        <button
                            className={activeMode === "efficiency" ? "btn-primary" : "btn-secondary"}
                            onClick={() => invoke("set_operation_mode", { mode: "efficiency" }).then(() => notify("Efficiency Locked")).catch(console.error)}
                            style={{ flex: 1, padding: '16px' }}
                        >🌱 Efficiency</button>
                    </div>
                </div>

                <div className="glass-card">
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
                        <div className="label">Advanced Peripheral Savers</div>
                        <span className="status-pill" style={{ background: 'var(--brand-muted)', color: 'var(--brand-accent)' }}>Hardware Hooks</span>
                    </div>

                    <table className="vitals-table">
                        <thead>
                            <tr>
                                <th>Sub-System</th>
                                <th>Status</th>
                                <th>Control</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td>
                                    <div style={{ fontWeight: '600' }}>USB Autosuspend</div>
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Powers down idle USB bus controllers.</div>
                                </td>
                                <td>
                                    <span className="status-pill" style={{ 
                                        background: metrics.config?.usb_autosuspend ? 'rgba(0, 255, 136, 0.1)' : 'rgba(255, 255, 255, 0.05)',
                                        color: metrics.config?.usb_autosuspend ? 'var(--success)' : 'var(--text-secondary)'
                                    }}>
                                        {metrics.config?.usb_autosuspend ? "ACTIVE" : "DISABLED"}
                                    </span>
                                </td>
                                <td>
                                    <button
                                        className={metrics.config?.usb_autosuspend ? "btn-primary" : "btn-secondary"}
                                        onClick={() => invoke("set_usb_autosuspend", { enabled: !metrics.config?.usb_autosuspend })
                                            .then(() => notify(`USB Autosuspend ${!metrics.config?.usb_autosuspend ? 'Enabled' : 'Disabled'}`))
                                            .catch(console.error)}
                                        style={{ padding: '6px 16px', fontSize: '11px' }}
                                    >{metrics.config?.usb_autosuspend ? "Disable" : "Enable"}</button>
                                </td>
                            </tr>
                            <tr>
                                <td>
                                    <div style={{ fontWeight: '600' }}>SATA ALPM</div>
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Aggressive Link Power Management for storage.</div>
                                </td>
                                <td>
                                    <span className="status-pill" style={{ 
                                        background: metrics.config?.sata_alpm ? 'rgba(0, 255, 136, 0.1)' : 'rgba(255, 255, 255, 0.05)',
                                        color: metrics.config?.sata_alpm ? 'var(--success)' : 'var(--text-secondary)'
                                    }}>
                                        {metrics.config?.sata_alpm ? "ACTIVE" : "DISABLED"}
                                    </span>
                                </td>
                                <td>
                                    <button
                                        className={metrics.config?.sata_alpm ? "btn-primary" : "btn-secondary"}
                                        onClick={() => invoke("set_sata_alpm", { enabled: !metrics.config?.sata_alpm })
                                            .then(() => notify(`SATA ALPM ${!metrics.config?.sata_alpm ? 'Enabled' : 'Disabled'}`))
                                            .catch(console.error)}
                                        style={{ padding: '6px 16px', fontSize: '11px' }}
                                    >{metrics.config?.sata_alpm ? "Disable" : "Enable"}</button>
                                </td>
                            </tr>
                            <tr>
                                <td>
                                    <div style={{ fontWeight: '600' }}>WiFi Radio</div>
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Hardware radio control via rfkill.</div>
                                </td>
                                <td>
                                    <span className="status-pill" style={{ 
                                        background: metrics.config?.wifi_enabled ? 'rgba(0, 255, 136, 0.1)' : 'rgba(255, 255, 255, 0.05)',
                                        color: metrics.config?.wifi_enabled ? 'var(--success)' : 'var(--text-secondary)'
                                    }}>
                                        {metrics.config?.wifi_enabled ? "ON" : "OFF"}
                                    </span>
                                </td>
                                <td>
                                    <button
                                        className={metrics.config?.wifi_enabled ? "btn-primary" : "btn-secondary"}
                                        onClick={() => invoke("set_wifi_enabled", { enabled: !metrics.config?.wifi_enabled })
                                            .then(() => notify(`WiFi ${!metrics.config?.wifi_enabled ? 'Enabled' : 'Disabled'}`))
                                            .catch(console.error)}
                                        style={{ padding: '6px 16px', fontSize: '11px' }}
                                    >{metrics.config?.wifi_enabled ? "Disable" : "Enable"}</button>
                                </td>
                            </tr>
                            <tr>
                                <td>
                                    <div style={{ fontWeight: '600' }}>Bluetooth</div>
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Wireless radio control for peripherals.</div>
                                </td>
                                <td>
                                    <span className="status-pill" style={{ 
                                        background: metrics.config?.bluetooth_enabled ? 'rgba(0, 255, 136, 0.1)' : 'rgba(255, 255, 255, 0.05)',
                                        color: metrics.config?.bluetooth_enabled ? 'var(--success)' : 'var(--text-secondary)'
                                    }}>
                                        {metrics.config?.bluetooth_enabled ? "ON" : "OFF"}
                                    </span>
                                </td>
                                <td>
                                    <button
                                        className={metrics.config?.bluetooth_enabled ? "btn-primary" : "btn-secondary"}
                                        onClick={() => invoke("set_bluetooth_enabled", { enabled: !metrics.config?.bluetooth_enabled })
                                            .then(() => notify(`Bluetooth ${!metrics.config?.bluetooth_enabled ? 'Enabled' : 'Disabled'}`))
                                            .catch(console.error)}
                                        style={{ padding: '6px 16px', fontSize: '11px' }}
                                    >{metrics.config?.bluetooth_enabled ? "Disable" : "Enable"}</button>
                                </td>
                            </tr>
                            <tr>
                                <td>
                                    <div style={{ fontWeight: '600' }}>PCIe ASPM</div>
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Active State Power Management for bus lines.</div>
                                </td>
                                <td>
                                    <span className="status-pill" style={{ 
                                        background: metrics.config?.pcie_aspm ? 'rgba(0, 255, 136, 0.1)' : 'rgba(255, 255, 255, 0.05)',
                                        color: metrics.config?.pcie_aspm ? 'var(--success)' : 'var(--text-secondary)'
                                    }}>
                                        {metrics.config?.pcie_aspm ? "ACTIVE" : "DISABLED"}
                                    </span>
                                </td>
                                <td>
                                    <button
                                        className={metrics.config?.pcie_aspm ? "btn-primary" : "btn-secondary"}
                                        onClick={() => invoke("set_pcie_aspm", { enabled: !metrics.config?.pcie_aspm })
                                            .then(() => notify(`PCIe ASPM ${!metrics.config?.pcie_aspm ? 'Enabled' : 'Disabled'}`))
                                            .catch(console.error)}
                                        style={{ padding: '6px 16px', fontSize: '11px' }}
                                    >{metrics.config?.pcie_aspm ? "Disable" : "Enable"}</button>
                                </td>
                            </tr>
                            <tr>
                                <td>
                                    <div style={{ fontWeight: '600' }}>NMI Watchdog</div>
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Kernel-level hardware hang detector.</div>
                                </td>
                                <td>
                                    <span className="status-pill" style={{ 
                                        background: metrics.config?.nmi_watchdog ? 'rgba(0, 255, 136, 0.1)' : 'rgba(255, 255, 255, 0.05)',
                                        color: metrics.config?.nmi_watchdog ? 'var(--success)' : 'var(--text-secondary)'
                                    }}>
                                        {metrics.config?.nmi_watchdog ? "ENABLED" : "QUIET"}
                                    </span>
                                </td>
                                <td>
                                    <button
                                        className={metrics.config?.nmi_watchdog ? "btn-primary" : "btn-secondary"}
                                        onClick={() => invoke("set_nmi_watchdog", { enabled: !metrics.config?.nmi_watchdog })
                                            .then(() => notify(`Watchdog ${!metrics.config?.nmi_watchdog ? 'Enabled' : 'Disabled'}`))
                                            .catch(console.error)}
                                        style={{ padding: '6px 16px', fontSize: '11px' }}
                                    >{metrics.config?.nmi_watchdog ? "Disable" : "Enable"}</button>
                                </td>
                            </tr>
                            <tr>
                                <td>
                                    <div style={{ fontWeight: '600' }}>VM Writeback</div>
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Advanced cache-to-disk flushing policy.</div>
                                </td>
                                <td>
                                    <span className="status-pill" style={{ 
                                        background: metrics.config?.vm_writeback ? 'rgba(0, 255, 136, 0.1)' : 'rgba(255, 255, 255, 0.05)',
                                        color: metrics.config?.vm_writeback ? 'var(--success)' : 'var(--text-secondary)'
                                    }}>
                                        {metrics.config?.vm_writeback ? "POWER-SAVE" : "STANDARD"}
                                    </span>
                                </td>
                                <td>
                                    <button
                                        className={metrics.config?.vm_writeback ? "btn-primary" : "btn-secondary"}
                                        onClick={() => invoke("set_vm_writeback", { enabled: !metrics.config?.vm_writeback })
                                            .then(() => notify(`WM Writeback ${!metrics.config?.vm_writeback ? 'Aggressive' : 'Standard'}`))
                                            .catch(console.error)}
                                        style={{ padding: '6px 16px', fontSize: '11px' }}
                                    >{metrics.config?.vm_writeback ? "Standard" : "Aggressive"}</button>
                                </td>
                            </tr>
                            <tr>
                                <td>
                                    <div style={{ fontWeight: '600' }}>SMT Status</div>
                                    <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>Hyper-threading / Sequential Multi-Threading.</div>
                                </td>
                                <td>
                                    <span className="status-pill" style={{ 
                                        background: metrics.config?.smt_status ? 'rgba(0, 255, 136, 0.1)' : 'rgba(255, 255, 255, 0.05)',
                                        color: metrics.config?.smt_status ? 'var(--success)' : 'var(--text-secondary)'
                                    }}>
                                        {metrics.config?.smt_status ? "ON" : "OFF"}
                                    </span>
                                </td>
                                <td>
                                    <button
                                        className={metrics.config?.smt_status ? "btn-primary" : "btn-secondary"}
                                        onClick={() => invoke("set_smt_status", { enabled: !metrics.config?.smt_status })
                                            .then(() => notify(`SMT ${!metrics.config?.smt_status ? 'Enabled' : 'Disabled'}`))
                                            .catch(console.error)}
                                        style={{ padding: '6px 16px', fontSize: '11px' }}
                                    >{metrics.config?.smt_status ? "Disable" : "Enable"}</button>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <div className="glass-card" style={{ marginTop: '24px' }}>
                    <div className="label">Micro-Architecture SMT Topology</div>
                    <div style={{ 
                        marginTop: '20px', 
                        display: 'grid', 
                        gridTemplateColumns: 'repeat(auto-fit, minmax(100px, 1fr))', 
                        gap: '12px' 
                    }}>
                        {metrics.cores.map((c, i) => {
                            const isPair = i % 2 !== 0; // Simplified SMT detection
                            return (
                                <div key={c.id} style={{
                                    padding: '12px',
                                    background: 'rgba(255,255,255,0.02)',
                                    borderRadius: '8px',
                                    border: isPair ? '1px dashed var(--border)' : '1px solid var(--border)',
                                    textAlign: 'center'
                                }}>
                                    <div style={{ fontSize: '9px', fontWeight: '800', opacity: 0.5 }}>
                                        {isPair ? `THREAD ${c.id}` : `CORE ${c.id}`}
                                    </div>
                                    <div style={{ fontSize: '14px', fontWeight: '800', marginTop: '4px' }}>
                                        {c.governor}
                                    </div>
                                    {c.frequency > c.max_frequency * 0.9 && (
                                        <div style={{ fontSize: '8px', color: 'var(--thermal-hot)', fontWeight: '900', marginTop: '4px' }}>
                                            ⚠️ PRESSURE
                                        </div>
                                    )}
                                </div>
                            );
                        })}
                    </div>
                </div>
            </div>

            <div className="side-pane glass-card">
                <div className="label">Thermal Vitals</div>
                <div style={{ marginTop: '20px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
                    <div className="mini-stat">
                        <span className="label">Package Temp</span>
                        <span className="val" style={{ color: (metrics.cpu_temperature || 0) > 80 ? 'var(--thermal-hot)' : 'var(--text-main)' }}>
                            {metrics.cpu_temperature ? `${metrics.cpu_temperature.toFixed(1)}°C` : 'N/A'}
                        </span>
                    </div>
                    <div className="mini-stat">
                        <span className="label">Freq Ceiling</span>
                        <span className="val">{Math.max(...metrics.cores.map(c => c.max_frequency))} MHz</span>
                    </div>
                    <div className="mini-stat">
                        <span className="label">Throttle Events</span>
                        <span className="val">{metrics.cores.filter(c => c.frequency > c.max_frequency * 0.95).length}</span>
                    </div>
                </div>
                <div style={{ marginTop: '24px', fontSize: '11px', color: 'var(--text-secondary)', fontStyle: 'italic' }}>
                    SMT pairs share physical execution units. WattWise optimizes these pairs to reduce context-switch overhead under power-saving tiers.
                </div>
            </div>
        </div>
    );
};
