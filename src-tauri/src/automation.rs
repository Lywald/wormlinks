use windows::Win32::UI::Accessibility::{
    IUIAutomation, CUIAutomation, IUIAutomationElement, TreeScope_Subtree, 
    UIA_ValuePatternId, IUIAutomationValuePattern, UIA_TextPatternId, IUIAutomationTextPattern,
    UIA_LegacyIAccessiblePatternId, IUIAutomationLegacyIAccessiblePattern
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{WindowFromPoint, GetWindowThreadProcessId, GetWindowRect, GetAncestor, GA_ROOT};
use windows::Win32::Foundation::RECT;
use windows::core::Interface;
use regex::Regex;

pub struct AutomationManager {
    automation: IUIAutomation,
    re: Regex,
    own_process_id: u32,
}

unsafe impl Send for AutomationManager {}
unsafe impl Sync for AutomationManager {}

impl AutomationManager {
    pub fn new() -> anyhow::Result<Self> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
            let re = Regex::new(r"wormlink://[^\s]+").unwrap();
            let own_process_id = std::process::id();
            Ok(Self { automation, re, own_process_id })
        }
    }

    pub fn find_matches_near_cursor(&self, point: windows::Win32::Foundation::POINT) -> Vec<crate::WormlinkMatch> {
        let mut matches = Vec::new();
        unsafe {
            let hwnd = WindowFromPoint(point);
            if hwnd.0 == std::ptr::null_mut() { return matches; }

            let mut process_id: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));
            if process_id == self.own_process_id { return matches; }

            // Get TRUE ROOT window for maximize dimensions (Multi-monitor safe)
            let root_hwnd = GetAncestor(hwnd, GA_ROOT);
            let mut parent_rect = RECT::default();
            let _ = GetWindowRect(root_hwnd, &mut parent_rect);

            let root_element = match self.automation.ElementFromHandle(hwnd) {
                Ok(el) => el,
                Err(_) => return matches,
            };

            let condition = match self.automation.CreateTrueCondition() {
                Ok(c) => c,
                Err(_) => return matches,
            };

            let elements = match root_element.FindAll(TreeScope_Subtree, &condition) {
                Ok(e) => e,
                Err(_) => return matches,
            };

            let count = elements.Length().unwrap_or(0);
            for i in 0..count {
                if let Ok(element) = elements.GetElement(i) {
                    if let Ok(offscreen) = element.CurrentIsOffscreen() {
                        if offscreen.as_bool() { continue; }
                    }

                    if let Ok(pattern) = element.GetCurrentPattern(UIA_TextPatternId) {
                        if let Ok(text_pattern) = pattern.cast::<IUIAutomationTextPattern>() {
                            self.collect_text_pattern_matches(&text_pattern, &element, &parent_rect, &mut matches);
                        }
                    }

                    if matches.is_empty() {
                        if let Ok(val) = element.GetCurrentPattern(UIA_ValuePatternId).and_then(|p| p.cast::<IUIAutomationValuePattern>()) {
                            if let Ok(v) = val.CurrentValue() {
                                self.collect_basic_matches(&v.to_string(), &element, &parent_rect, &mut matches);
                            }
                        }
                        if let Ok(name) = element.CurrentName() {
                            self.collect_basic_matches(&name.to_string(), &element, &parent_rect, &mut matches);
                        }
                        
                        // Fallback for LibreOffice / Legacy apps
                        if matches.is_empty() {
                            if let Ok(legacy) = element.GetCurrentPattern(UIA_LegacyIAccessiblePatternId).and_then(|p| p.cast::<IUIAutomationLegacyIAccessiblePattern>()) {
                                if let Ok(name) = legacy.CurrentName() {
                                    self.collect_basic_matches(&name.to_string(), &element, &parent_rect, &mut matches);
                                }
                                if let Ok(val) = legacy.CurrentValue() {
                                    self.collect_basic_matches(&val.to_string(), &element, &parent_rect, &mut matches);
                                }
                            }
                        }
                    }
                }
            }
        }
        matches
    }

    fn collect_text_pattern_matches(&self, pattern: &IUIAutomationTextPattern, element: &IUIAutomationElement, parent_rect: &RECT, matches: &mut Vec<crate::WormlinkMatch>) {
        unsafe {
            let range = match pattern.DocumentRange() { Ok(r) => r, Err(_) => return };
            let text = range.GetText(-1).unwrap_or_default().to_string();
            for caps in self.re.captures_iter(&text) {
                let url = caps.get(0).unwrap().as_str();
                if let Ok(found_range) = range.FindText(&windows::core::BSTR::from(url), false, false) {
                    if let Ok(rects_ptr) = found_range.GetBoundingRectangles() {
                        let l_bound = windows::Win32::System::Ole::SafeArrayGetLBound(rects_ptr, 1).unwrap_or(0);
                        let u_bound = windows::Win32::System::Ole::SafeArrayGetUBound(rects_ptr, 1).unwrap_or(-1);
                        if u_bound >= l_bound {
                            let mut left: f64 = 0.0; let mut top: f64 = 0.0; let mut w: f64 = 0.0; let mut h: f64 = 0.0;
                            let _ = windows::Win32::System::Ole::SafeArrayGetElement(rects_ptr, &0i32, &mut left as *mut _ as *mut _);
                            let _ = windows::Win32::System::Ole::SafeArrayGetElement(rects_ptr, &1i32, &mut top as *mut _ as *mut _);
                            let _ = windows::Win32::System::Ole::SafeArrayGetElement(rects_ptr, &2i32, &mut w as *mut _ as *mut _);
                            let _ = windows::Win32::System::Ole::SafeArrayGetElement(rects_ptr, &3i32, &mut h as *mut _ as *mut _);
                            
                            if left != 0.0 || top != 0.0 {
                                matches.push(crate::WormlinkMatch {
                                    url: url.to_string(),
                                    source_app: self.get_app_name(element),
                                    x: left,
                                    y: top + h, 
                                    width: 350.0,
                                    height: 280.0,
                                    trigger_x: left,
                                    trigger_y: top,
                                    trigger_w: w,
                                    trigger_h: h,
                                    is_demo: false,
                                    detection_method: "window_text".to_string(),
                                    parent_x: parent_rect.left as f64,
                                    parent_y: parent_rect.top as f64,
                                    parent_width: (parent_rect.right - parent_rect.left) as f64,
                                    parent_height: (parent_rect.bottom - parent_rect.top) as f64,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    fn collect_basic_matches(&self, text: &str, element: &IUIAutomationElement, parent_rect: &RECT, matches: &mut Vec<crate::WormlinkMatch>) {
        for caps in self.re.captures_iter(text) {
            let url = caps.get(0).unwrap().as_str().to_string();
            unsafe {
                if let Ok(rect) = element.CurrentBoundingRectangle() {
                    let w = (rect.right - rect.left) as f64;
                    let h = (rect.bottom - rect.top) as f64;
                    matches.push(crate::WormlinkMatch {
                        url,
                        source_app: self.get_app_name(element),
                        x: rect.left as f64,
                        y: rect.bottom as f64,
                        width: 350.0,
                        height: 280.0,
                        trigger_x: rect.left as f64,
                        trigger_y: rect.top as f64,
                        trigger_w: w,
                        trigger_h: h,
                        is_demo: false,
                        detection_method: "window_basic".to_string(),
                        parent_x: parent_rect.left as f64,
                        parent_y: parent_rect.top as f64,
                        parent_width: (parent_rect.right - parent_rect.left) as f64,
                        parent_height: (parent_rect.bottom - parent_rect.top) as f64,
                    });
                }
            }
        }
    }

    fn get_app_name(&self, element: &IUIAutomationElement) -> String {
        unsafe {
            if let Ok(process_id) = element.CurrentProcessId() {
                let process_id = process_id as u32;
                // Try to get process name from PID
                if let Ok(handle) = windows::Win32::System::Threading::OpenProcess(
                    windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION,
                    false,
                    process_id
                ) {
                    let mut buffer = [0u16; 260];
                    let mut size = buffer.len() as u32;
                    if windows::Win32::System::Threading::QueryFullProcessImageNameW(
                        handle,
                        windows::Win32::System::Threading::PROCESS_NAME_NATIVE,
                        windows::core::PWSTR::from_raw(buffer.as_mut_ptr()),
                        &mut size
                    ).is_ok() {
                        let path = String::from_utf16_lossy(&buffer[..size as usize]);
                        if let Some(name) = std::path::Path::new(&path).file_name() {
                            return name.to_string_lossy().to_string();
                        }
                    }
                }
            }
            element.CurrentName().map(|n| n.to_string()).unwrap_or_else(|_| "Unknown App".to_string())
        }
    }
}
