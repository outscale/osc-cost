# SPDX-FileCopyrightText: 2023 Outscale SAS
# SPDX-License-Identifier: BSD-3-Clause
{{- define "osccost.fullname" -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "osccost.service" -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s-service" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "osccost.deployment" -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s-deployment" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "osccost.ingress" -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s-ingress" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "osccost.secret" -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s-secret" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "osccost.serviceMonitor" -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s-servicemonitor" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "osccost.podDisruptionBudget" -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s-pdb" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "osccost.serviceAccount" -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s-sa" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
