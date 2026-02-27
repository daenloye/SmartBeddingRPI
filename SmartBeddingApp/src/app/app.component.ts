import { Component, inject } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { AlertPanelComponent } from "./components/alert/alert.component";
import { ApiService } from './services/api.service';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, AlertPanelComponent],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent {
  title = 'SmartBeddingSystem';


}
