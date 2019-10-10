pub trait StringUtils {
    fn substring(&self, start: usize, len: usize) -> Self;
    fn remove_whitespace(&mut self) -> Self; 
}


