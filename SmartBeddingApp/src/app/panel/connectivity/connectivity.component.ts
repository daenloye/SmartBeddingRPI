import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ApiService } from '../../services/api.service';
import { ConnectivityAnswer } from '../../interfaces/connectivity-answer';
import { timer, Subscription, of } from 'rxjs';
import { switchMap, catchError } from 'rxjs/operators';

@Component({
  selector: 'app-connectivity',
  standalone: true,
  imports: [CommonModule], // Puedes quitarlo si no usas pipes como | async o | date
  templateUrl: './connectivity.component.html',
  styleUrl: './connectivity.component.css'
})
export class ConnectivityComponent implements OnInit, OnDestroy {

  data?: ConnectivityAnswer;
  loading: boolean = true;
  refreshing: boolean = false;
  private refreshSub?: Subscription;

  constructor(private apiService: ApiService) { }

  ngOnInit(): void {
    this.refreshSub = timer(0, 8000).pipe(
      switchMap(() => {
        this.refreshing = true;
        return this.apiService.getConnectivity().pipe(
          catchError(err => {
            console.error("Error en polling:", err);
            return of(null);
          })
        );
      })
    ).subscribe({
      next: (response) => {
        if (response?.result && response.data) {
          this.data = response.data;
        }
        this.loading = false;
        this.refreshing = false;
      }
    });
  }

  ngOnDestroy(): void {
    this.refreshSub?.unsubscribe();
  }
}
