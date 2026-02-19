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

## 완료

이제 im-not-notion에서 아래 정보만 입력하면 됩니다:
- **Host**: 서버 IP (Docker인 경우 `localhost`)
- **Port**: 22 (Docker인 경우 매핑한 포트, e.g., `2222`)
- **Username**: 생성한 유저명
- **Password**: 생성 시 입력한 비밀번호
