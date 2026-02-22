# Server Initial Setup Guide

> im-not-notion 사용 전 서버 초기 세팅 가이드
>
> 목표: SSH 접속 가능한 일반 유저 계정 생성 + root SSH 접속 차단

## 1. SSH 서버 설치

대부분의 클라우드 서버(AWS, GCP, Vultr 등)에는 이미 설치되어 있음.
설치가 안 되어 있는 경우:

```bash
# Debian / Ubuntu
apt update && apt install -y openssh-server

# CentOS / RHEL / Fedora
yum install -y openssh-server
# 또는
dnf install -y openssh-server

# 서비스 시작 + 부팅 시 자동 시작
systemctl enable --now sshd
```

설치 확인:
```bash
systemctl status sshd
```

## 2. 유저 생성

```bash
adduser myuser
# 비밀번호 입력 (im-not-notion SSH 설정에 사용할 비밀번호)
```

이것만으로 해당 유저로 SSH 접속이 가능합니다.
sudo 그룹 추가는 필요 없습니다.

## 3. 새 유저로 SSH 접속 테스트

root SSH를 막기 전에 반드시 새 유저로 접속이 되는지 확인:

```bash
# 로컬 터미널에서
ssh myuser@서버IP
```

**접속 확인 후** 다음 단계로 진행하세요. 이 단계를 건너뛰면 서버에 접속할 수 없게 될 수 있습니다.

## 4. root SSH 접속 차단

```bash
# root 상태에서 sshd 설정 편집
vi /etc/ssh/sshd_config
```

아래 항목을 찾아 수정 (주석 처리되어 있으면 주석 해제):
```
PermitRootLogin no
```

설정 적용:
```bash
systemctl restart sshd
```

---

## Docker 환경인 경우

Docker 컨테이너는 systemd가 없어서 `systemctl` 명령이 작동하지 않음.
아래 두 가지 방법 중 선택.

### 방법 A: docker-compose로 새로 구성

```yaml
# docker-compose.yml
services:
  inn-server:
    image: ubuntu:24.04
    ports:
      - "2222:22"
    command: >
      bash -c "
        apt update &&
        apt install -y openssh-server &&
        mkdir -p /run/sshd &&
        useradd -m -s /bin/bash myuser &&
        echo 'myuser:mypassword' | chpasswd &&
        sed -i 's/#PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config &&
        /usr/sbin/sshd -D
      "
```

```bash
docker compose up -d
ssh myuser@localhost -p 2222
```

### 방법 B: 이미 있는 컨테이너에 sshd 추가

컨테이너 안에서 SSH를 설치한 뒤, root의 `.bashrc`에 자동 시작 등록:

```bash
# 컨테이너 안에서 (root)
apt update && apt install -y openssh-server
mkdir -p /run/sshd

# 유저 생성
adduser myuser

# root .bashrc에 sshd 자동 시작 추가
echo '/usr/sbin/sshd' >> ~/.bashrc
```

컨테이너가 bash로 시작되면 root 쉘이 열리면서 sshd가 자동으로 뜸.
단, 컨테이너 포트 매핑(`-p 2222:22`)은 컨테이너 생성 시에만 설정 가능.

---

## 5. 필수 패키지 설치

im-not-notion이 서버에서 사용하는 도구들입니다. 앱의 Hugo 설치 마법사가 `curl`, `tar`, `git` 존재 여부를 자동 체크합니다.

```bash
# Debian / Ubuntu
apt update && apt install -y curl tar git cron python3

# CentOS / RHEL / Fedora
dnf install -y curl tar git cronie python3
```

| 패키지 | 용도 | 필수 여부 |
|--------|------|-----------|
| **curl** | Hugo 다운로드, GitHub API 호출 | 필수 (앱 체크) |
| **tar** | Hugo 압축 해제 | 필수 (앱 체크) |
| **git** | 사이트 초기화, 테마 설치, 플러그인 push/pull | 필수 (앱 체크) |
| **cron** | 플러그인 예약 실행 (auto-push, backup 등) | 플러그인 cron 사용 시 |
| **python3** | Python 플러그인 실행 | 플러그인 사용 시 |

### 플러그인별 추가 패키지

일부 플러그인은 Python 패키지가 필요합니다:

```bash
# web-clipper 플러그인
pip3 install --user requests html2text

# ai-draft 플러그인
pip3 install --user openai
# + 환경변수: export OPENAI_API_KEY="sk-..."
```

---

## 6. cron 서비스 설정

플러그인의 예약 실행(auto-push, backup 등)에 cron이 필요합니다.

```bash
# 설치 확인
which crontab

# 서비스 시작
systemctl enable --now cron    # systemd 환경
# 또는
service cron start             # Docker 등 non-systemd 환경
```

### Docker 환경

Docker에서는 cron이 자동 시작되지 않습니다. root `.bashrc`에 추가:

```bash
echo 'pgrep -x cron > /dev/null || /usr/sbin/cron -P' >> ~/.bashrc
```

> sshd도 같은 패턴을 권장합니다:
> ```bash
> pgrep -x sshd > /dev/null || /usr/sbin/sshd
> ```

---

## 7. SSH 키 설정 (Git Push용)

플러그인(git-autopush 등)이 서버에서 `git push`를 하려면 SSH 키가 필요합니다.

```bash
# 서버에서 (일반 유저로)
ssh-keygen -t ed25519 -C "inn-autopush-plugin"

# 공개 키 확인
cat ~/.ssh/id_ed25519.pub
```

이 공개 키를 Git 호스팅에 등록:
- **GitHub**: Settings > SSH and GPG keys > New SSH key
- **GitLab**: Preferences > SSH Keys

등록 후 연결 테스트:
```bash
ssh -T git@github.com
```

---

## 8. 서버 디렉토리 구조

앱 사용 시 자동 생성되는 디렉토리/파일:

```
~/
├── .inn_server_config.json    # 서버 설정 (앱이 자동 생성)
├── .inn_plugins/              # 설치된 플러그인
│   └── plugin-name/
│       ├── plugin.json
│       ├── main.py
│       └── .disabled          # 비활성화 마커 (있으면 비활성)
├── .local/bin/
│   └── hugo                   # Hugo 바이너리 (앱 설치 마법사로 설치)
├── .ssh/
│   ├── id_ed25519             # SSH 키 (git push용)
│   └── id_ed25519.pub
├── inn_backups/               # blog-backup 플러그인 백업 위치
└── [hugo-site]/               # Hugo 사이트 루트
    ├── content/
    ├── static/
    └── themes/
```

---

## 완료

이제 im-not-notion에서 아래 정보만 입력하면 됩니다:
- **Host**: 서버 IP (Docker인 경우 `localhost`)
- **Port**: 22 (Docker인 경우 매핑한 포트, e.g., `2222`)
- **Username**: 생성한 유저명
- **Password**: 생성 시 입력한 비밀번호
