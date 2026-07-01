# Nemesis Operational Runbook

## Kill Switch Activation

1. Check Grafana dashboard for `nemesis_kill_switch_active`
2. If active, query audit_log:
   `SELECT * FROM audit_log WHERE event_type='kill_switch' ORDER BY receive_ts DESC LIMIT 5;`
3. Resolve root cause before resetting
4. Reset via config change + pod restart (NOT API call)

## Stale Feed Recovery

1. Verify WebSocket reconnection metric: `nemesis_ws_reconnections_total`
2. Check exchange status page for reported outages
3. If persistent >5min, restart pod
4. Verify `bars_processed` resumes after restart

## Reconciliation Drift Response

1. Query drift events:
   `SELECT * FROM audit_log WHERE event_type='reconciliation_drift' ORDER BY receive_ts DESC;`
2. Compare local vs exchange positions manually
3. If orphaned orders found, cancel via exchange UI
4. File incident report with audit_log evidence

## Emergency Shutdown

1. Scale deployment to 0: `kubectl scale deployment/nemesis --replicas=0`
2. Verify all open orders canceled via exchange
3. Check final audit_log entries
4. Notify team via alert channel

## Secret Rotation

1. Update AWS Secrets Manager entry
2. Restart pod (secrets fetched at startup only)
3. Verify health check passes
4. Monitor for auth errors in logs

## Post-Mortem Checklist

- Collect all audit_log entries for affected time window
- Export Grafana dashboard snapshot for the incident period
- Download latest database backup before any remediation
- File incident report with timeline, impact, and root cause
