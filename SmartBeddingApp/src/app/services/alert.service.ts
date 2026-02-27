// src/app/services/alert.service.ts
import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable, timer } from 'rxjs';
import { take } from 'rxjs/operators';

export interface Alert {
  id?: string;
  message: string;
  type: 'info' | 'success' | 'warning' | 'error';
  autoClose?: boolean;
  duration?: number; // duration in milliseconds
}

@Injectable({
  providedIn: 'root'
})
export class AlertService {
  private alertsSubject: BehaviorSubject<Alert[]> = new BehaviorSubject<Alert[]>([]);
  public alerts$: Observable<Alert[]> = this.alertsSubject.asObservable();

  constructor() {}

  /**
   * Adds an alert to the queue.
   * @param alert The alert object.
   */
  addAlert(alert: Alert) {
    const alerts = this.alertsSubject.getValue();
    const newAlert: Alert = { ...alert, id: this.generateId() };
    this.alertsSubject.next([...alerts, newAlert]);


    if (newAlert.autoClose !== false) { // Default to autoClose true
      const duration = newAlert.duration || 5000; // Default 5 seconds
      timer(duration).pipe(take(1)).subscribe(() => {
        this.removeAlert(newAlert.id!);
      });
    }
  }

  /**
   * Removes an alert by its ID.
   * @param id The ID of the alert to remove.
   */
  removeAlert(id: string) {
    const alerts = this.alertsSubject.getValue().filter(alert => alert.id !== id);
    this.alertsSubject.next(alerts);
  }

  /**
   * Clears all alerts.
   */
  clearAlerts() {
    this.alertsSubject.next([]);
  }

  private generateId(): string {
    return Math.random().toString(36).substring(2, 9);
  }
}
