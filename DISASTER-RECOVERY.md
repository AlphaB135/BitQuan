# BitQuan Disaster Recovery Guide

## 🚨 ภาพรวม Disaster Recovery

BitQuan มีระบบ backup และ disaster recovery แบบครบวงจรเพื่อให้ความพร้อมใช้งาน (business continuity) แม้ในกรณีที่เกิดความล้มเหลวระดับร้ายแรง

### 🎯 เป้าหมาย Disaster Recovery

- **RPO (Recovery Point Objective)**: สูงสุด 15 นาทีของข้อมูลที่สูญเสีย
- **RTO (Recovery Time Objective)**: สูงสุด 1 ชั่วโมงในการกลับมาทำงานปกติ
- **Data Loss Prevention**: การสูญเสียข้อมูลน้อยกว่า 0.1%
- **Service Availability**: 99.9% uptime หลังการ restore

## 🗂️ ประเภท Backup

### 1. **Full Backups**
- **ความถี่**: รันทุกวันเวลา 2:00 AM
- **ข้อมูล**: Configuration, blockchain data, logs
- **Compression**: GZIP compression
- **Encryption**: GPG encryption (AES-256)
- **Retention**: 30 วัน

### 2. **Incremental Backups**
- **ความถี่**: รันทุก 6 ชั่วโมง
- **ข้อมูล**: เฉพาะที่เปลี่ยนแปลงจาก backup ล่าสุด
- **Efficiency**: เร็วกกว่า full backup มาก
- **Dependencies**: ต้องมี full backup ล่าสุด

### 3. **Configuration-Only Backups**
- **ความถี่**: ทุกครั้งที่มีการเปลี่ยนแปลง configuration
- **ข้อมูล**: เฉพาะ configuration files และ settings
- **Speed**: เร็วยมาก
- **Use Case**: Quick rollback หลังการเปลี่ยนแปลง

## 📁 โครงสร้าง Backup Directory

```
/opt/backups/bitquan/
├── bitquan_backup_20241218_020000.tar.gz.gpg     # Daily full backup
├── bitquan_backup_20241218_080000.tar.gz.gpg     # Incremental backup
├── bitquan_backup_20241218_140000.tar.gz.gpg     # Incremental backup
├── bitquan_backup_20241218_200000.tar.gz.gpg     # Incremental backup
├── bitquan_backup_20241218_config.tar.gz.gpg      # Configuration backup
├── latest -> bitquan_backup_20241218_200000.tar.gz.gpg  # Symlink to latest
├── pre_recovery_20241218_150000.tar.gz             # Auto-backup before recovery
└── retention/
    ├── 2024-12/
    │   ├── bitquan_backup_20241201_*.tar.gz.gpg
    │   └── ...
    └── 2024-11/
        └── ...
```

## 🔧 การติดตั้ง Backup System

### Prerequisites

```bash
# Install required packages
sudo apt-get update
sudo apt-get install -y gnupg2 tar gzip gzip coreutils

# Create backup directories
sudo mkdir -p /opt/backups/bitquan
sudo mkdir -p /var/log/bitquan
sudo chown -R bitquan:bitquan /opt/backups/bitquan /var/log/bitquan
```

### Environment Configuration

```bash
# /etc/environment หรือ ~/.bashrc
export BACKUP_DIR="/opt/backups/bitquan"
export CONFIG_DIR="/etc/bitquan"
export DATA_DIR="/var/lib/bitquan"
export LOG_FILE="/var/log/bitquan/backup.log"
export RETENTION_DAYS=30
export COMPRESS=true
export ENCRYPT=true
export GPG_RECIPIENT="backup@bitquan.org"
export ALERT_WEBHOOK_URL="https://hooks.slack.com/your-webhook"
```

### GPG Key Setup

```bash
# Generate GPG key for encryption
gpg --batch --gen-key << EOF
Key-Type: RSA
Key-Length: 4096
Subkey-Type: RSA
Subkey-Length: 4096
Name-Real: BitQuan Backup
Name-Email: backup@bitquan.org
Expire-Date: 0
%no-protection
EOF

# Export public key for distribution
gpg --armor --export backup@bitquan.org > backup_public_key.asc

# Set up recipient for automation
gpg --import backup_public_key.asc
```

### Systemd Service Configuration

```ini
# /etc/systemd/system/bitquan-backup.service
[Unit]
Description=BitQuan Backup Service
After=bitquan.service
Requires=bitquan.service

[Service]
Type=oneshot
User=bitquan
Group=bitquan
ExecStart=/opt/bitquan/scripts/backup.sh --type full
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

```ini
# /etc/systemd/system/bitquan-backup.timer
[Unit]
Description=Run BitQuan Backup Daily
Requires=bitquan-backup.service

[Timer]
OnCalendar=*-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
# Enable backup service
sudo systemctl daemon-reload
sudo systemctl enable bitquan-backup.timer
sudo systemctl start bitquan-backup.timer
```

## 📋 Backup Procedures

### Daily Automated Backup

```bash
# Manual trigger (for testing)
sudo -u bitquan /opt/bitquan/scripts/backup.sh

# Check backup status
sudo systemctl status bitquan-backup.timer
sudo journalctl -u bitquan-backup.service -f
```

### Configuration Backup

```bash
# Quick configuration backup
sudo -u bitquan /opt/bitquan/scripts/backup.sh --type config-only

# Backup before making changes
sudo -u bitquan /opt/bitquan/scripts/backup.sh --type config-only
# Make configuration changes
```

### Incremental Backup

```bash
# Manual incremental backup
sudo -u bitquan /opt/bitquan/scripts/backup.sh --type incremental
```

## 🔄 Recovery Procedures

### 1. **Pre-Recovery Checklist**

ก่อนเริ่ม recovery:

- [ ] ตรวจสอบว่าเหตุการขัดข้องได้รับการแก้ไขแล้ว
- [ ] ระบุสาเหตุของความล้มเหลว
- [ ] มี backup ที่เหมาะสมสำหรับการ restore
- [ ] มีเวลา downtime ที่เหมาะสม
- [ => Inform stakeholders และ users]
- [ ] เตรียม rollback plan หากการ restore ล้มเหลว

### 2. **Disaster Recovery Steps**

#### Phase 1: Assessment (5-15 นาที)

```bash
# 1. ตรวจสอบความเสียหาย
sudo -u bitquan /opt/bitquan/scripts/health-check.sh

# 2. ตรวจสอบ available backups
sudo -u bitquan /opt/bitquan/scripts/recover.sh --list

# 3. ตรวจสอบ backup integrity
sudo -u bitquan /opt/bitquan/scripts/backup-verify.sh --latest
```

#### Phase 2: Recovery (15-45 นาที)

```bash
# 1. เลือก backup ที่จะ restore
BACKUP_FILE="bitquan_backup_20241218_020000.tar.gz.gpg"

# 2. ทำการ recovery
sudo -u bitquan /opt/bitquan/scripts/recover.sh --force $BACKUP_FILE
```

#### Phase 3: Verification (5-15 นาที)

```bash
# 1. ตรวจสอบ service status
sudo systemctl status bitquan

# 2. ตรวจสอบ blockchain sync
bitquan-cli getblockcount

# 3. ตรวจสอบ wallet functionality
bitquan-cli listwallets
```

#### Phase 4: Post-Recovery (5-30 นาที)

```bash
# 1. ตรวจสอบ logs
sudo journalctl -u bitquan --since "5 minutes ago" -f

# 2. ทดสอบ connectivity
curl -X POST http://localhost:8334 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'

# 3. สร้าง post-incident report
sudo -u bitquan /opt/bitquan/scripts/incident-report.sh
```

### 3. **Service-Specific Recovery**

#### RPC Service Recovery

```bash
# Restore RPC configuration specifically
sudo -u bitquan /opt/bitquan/scripts/recover.sh \
  bitquan_backup_20241218_config.tar.gz.gpg

# Verify RPC service
curl -X POST http://localhost:8334 \
  -H "Authorization: Basic $(echo -n 'user:pass' | base64)" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'
```

#### Wallet Recovery

```bash
# Restore from wallet backup
sudo -u bitquan cp /opt/backups/bitquan/wallet_backup.dat /var/lib/bitquan/wallets/

# Import wallet
bitquan-cli importwallet /var/lib/bitquan/wallets/backup_wallet.dat

# Verify wallet
bitquan-cli listwallets
bitquan-cli getbalance "*"
```

#### Blockchain Data Recovery

```bash
# Restore from full backup
sudo -u bitquan /opt/bitquan/scripts/recover.sh \
  bitquan_backup_20241218_full.tar.gz.gpg

# Verify blockchain state
bitquan-cli getblockchaininfo
bitquan-cli getblockcount
```

## 🚨 Emergency Procedures

### Complete System Failure

```bash
# 1. เริ่ม system ใหม่ใหม่
# 2. Install BitQuan package
sudo apt-get install bitquan

# 3. Restore from backup
sudo -u bitquan /opt/bitquan/scripts/recover.sh \
  /mnt/backups/bitquan_backup_latest.tar.gz.gpg

# 4. Start services
sudo systemctl start bitquan
```

### Data Corruption Detection

```bash
# Detect data corruption
bitquan-cli verifychain

# If corruption detected:
sudo systemctl stop bitquan
sudo -u bitquan /opt/bitquan/scripts/recover.sh \
  bitquan_backup_last_known_good.tar.gz.gpg
sudo systemctl start bitquan
```

### Ransomware Response

```bash
# 1. Isolate affected system
sudo iptables -I INPUT -s <attacker_ip> -j DROP
sudo systemctl stop bitquan

# 2. Restore from clean backup
sudo -u bitquan /opt/bitquan/scripts/recover.sh \
  bitquan_backup_pre_incident.tar.gz.gpg

# 3. Change all credentials
sudo -u bitquan /opt/bitquan/scripts/security-hardening.sh

# 4. Monitor for suspicious activity
sudo -u bitquan /opt/bitquan/scripts/security-monitor.sh
```

## 📊 Monitoring and Alerting

### Backup Monitoring

```bash
# Check backup success
tail -100 /var/log/bitquan/backup.log | grep -E "(ERROR|CRITICAL)"

# Check backup size
ls -lh /opt/backups/bitquan/*.tar.gz.gpg | tail -10

# Check backup age
find /opt/backups/bitquan -name "*.tar.gz.gpg" -mtime +1 -ls
```

### Recovery Metrics

```bash
# Recovery time tracking
time sudo -u bitquan /opt/bitquan/scripts/recover.sh backup.tar.gz.gpg

# Service availability after recovery
while ! systemctl is-active --quiet bitquan; do
    sleep 5
    echo "Waiting for service to start..."
done
```

### Alerting Configuration

```bash
# Backup failure alert
if ! sudo -u bitquan /opt/bitquan/scripts/backup.sh; then
    curl -X POST "$SLACK_WEBHOOK" \
      -H "Content-Type: application/json" \
      -d '{"text":"🚨 BitQuan backup failed!"}'
fi

# Recovery success alert
if sudo -u bitquan /opt/bitquan/scripts/recover.sh backup.tar.gz.gpg; then
    curl -X POST "$SLACK_WEBHOOK" \
      -H "Content-Type: application/json" \
      -d '{"text":"✅ BitQuan recovery successful!"}'
fi
```

## 🧪 Testing and Validation

### Backup Testing

```bash
# Test backup integrity
sudo -u bitquan /opt/bitquan/scripts/backup-test.sh

# Simulated restore test
sudo -u bitquan /opt/bitquan/scripts/recovery-test.sh \
  --dry-run --backup latest

# Full restore test (on staging)
sudo -u bitquan /opt/bitquan/scripts/recovery-test.sh \
  --staging --backup latest
```

### Disaster Recovery Drills

#### Monthly Drill Schedule

1. **Week 1**: Backup verification test
2. **Week 2**: Configuration restore test
3. **Week 3**: Full system restore test (staging)
4. **Week 4**: Ransomware response drill

```bash
# Automated monthly drill
sudo -u bitquan /opt/bitquan/scripts/disaster-drill.sh --monthly
```

## 📋 Maintenance Procedures

### Weekly Maintenance

```bash
# Check backup retention
find /opt/backups/bitquan -name "*.tar.gz.gpg" -mtime +30 -ls

# Clean up old temporary files
find /tmp -name "*bitquan_backup*" -mtime +1 -delete

# Verify GPG keys
gpg --list-keys backup@bitquan.org
```

### Monthly Maintenance

```bash
# Test restore procedures on staging
sudo -u bitquan /opt/bitquan/scripts/restore-staging-test.sh

# Update backup scripts
cd /opt/bitquan/scripts
git pull origin main

# Review and update backup policies
sudo -u bitquan /opt/bitquan/scripts/backup-policy-review.sh
```

### Quarterly Maintenance

```bash
# Full disaster recovery drill
sudo -u bitquan /opt/bitquan/scripts/full-dr-test.sh

# Review and update RTO/RPO
sudo -u bitquan /opt/bitquan/scripts/rto-rpo-review.sh

# Update documentation
sudo -u bitquan /opt/bitquan/scripts/update-docs.sh
```

## 📞 Contact and Support

### Emergency Contacts

- **Primary Support**: support@bitquan.org
- **Security Team**: security@bitquan.org
- **Infrastructure Team**: infra@bitquan.org

### Support Channels

- **Emergency Hotline**: +1-XXX-XXX-XXXX
- **Slack**: #bitquan-emergency
- **Email**: emergency@bitquan.org

### Support Tiers

#### Tier 1: Basic Support (1-4 ชั่วโมง)
- Basic troubleshooting
- Backup status inquiries
- Simple recovery procedures

#### Tier 2: Advanced Support (2-8 ชั่วโมง)
- Complex recovery scenarios
- System integrity checks
- Performance optimization

#### Tier 3: Emergency Support (Immediate)
- Complete system failure
- Security incidents
- Data corruption emergencies

## 📖 Documentation and Training

### Required Documentation

- [ ] **Backup Procedures Manual** - Step-by-step backup processes
- [ ] **Recovery Runbook** - Detailed recovery procedures
- [ ] **Incident Response Plan** - Security incident handling
- [ ] **Contact Directory** - Emergency contact information
- [ ] **System Architecture** - Infrastructure diagrams and dependencies

### Staff Training

#### Backup Operations Training
- Backup script usage
- Backup verification procedures
- Troubleshooting common issues
- Emergency response protocols

#### Recovery Training
- Recovery procedure execution
- Service restart procedures
- Data verification methods
- Post-recovery testing

### Documentation Updates

- **Weekly**: Backup logs and status reports
- **Monthly**: Recovery drill results and lessons learned
- **Quarterly**: Full disaster recovery plan review
- **Annually**: Complete backup and recovery strategy review

---

**สำคัญ**: Disaster recovery is a critical business function. Regular testing, maintenance, and updates are essential to ensure recovery effectiveness when needed.
