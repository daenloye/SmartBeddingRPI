import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { AlertPanelComponent } from "./components/alert/alert.component";

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, AlertPanelComponent],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent {
  title = 'SmartBeddingSystem';
}
