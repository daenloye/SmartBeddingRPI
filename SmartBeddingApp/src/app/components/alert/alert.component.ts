// src/app/components/alert-panel/alert-panel.component.ts
import { Component, OnInit, OnDestroy } from '@angular/core';
import { AlertService, Alert } from '../../services/alert.service';
import { Observable, Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { CommonModule } from '@angular/common'; // ¡Importante para ngFor!

@Component({
  selector: 'app-alert-panel',
  standalone: true, // Esto es clave en Angular 18
  imports: [CommonModule], // CommonModule es necesario para directivas como ngFor, ngIf
  templateUrl: './alert.component.html'
})
export class AlertPanelComponent implements OnInit, OnDestroy {
  alerts$!: Observable<Alert[]>;
  private destroy$ = new Subject<void>();

  constructor(private alertService: AlertService) {}

  ngOnInit() {
    this.alerts$ = this.alertService.alerts$.pipe(takeUntil(this.destroy$));
  }

  getAlertClasses(alert: Alert): string {
    return `alert alert-${alert.type} flex items-center justify-between shadow-lg mb-2`;
  }

  removeAlert(id: string) {
    this.alertService.removeAlert(id);
  }

  ngOnDestroy() {
    this.destroy$.next();
    this.destroy$.complete();
  }
}
